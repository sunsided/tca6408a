//! Per-pin handles implementing the `embedded-hal` 1.0 digital traits.
//!
//! [`Tca6408a::into_shared`] moves the driver into a mutex of your choice, and
//! [`Shared::split`] hands out the pins `P0` to `P7`. Each [`Pin`] implements
//! [`InputPin`], [`OutputPin`] and [`StatefulOutputPin`].
//!
//! ```
//! # use core::cell::RefCell;
//! # use tca6408a_sim::{FakeI2c, Tca6408aModel};
//! use embedded_hal::digital::{InputPin, OutputPin};
//! use tca6408a::Tca6408a;
//!
//! # let model = RefCell::new(Tca6408aModel::new());
//! # let i2c = FakeI2c::new(&model);
//! let io = Tca6408a::new(i2c).into_shared::<RefCell<_>>()?;
//! let pins = io.split();
//!
//! let mut vci_en = pins.p7;
//! vci_en.make_output(true)?; // latch first, then direction
//! vci_en.set_low()?;
//!
//! let mut bot = pins.p0;
//! let pressed = bot.is_low()?;
//! # Ok::<(), tca6408a::pins::PinError<embedded_hal::i2c::ErrorKind>>(())
//! ```
//!
//! ## Shared state
//!
//! The pins share the driver and a cache of the Output and Configuration registers through
//! a [`PortMutex`]. Changing one pin is a read-modify-write of a whole register; the cache
//! makes it a single I²C write, and the mutex keeps two pins from racing. This crate
//! implements [`PortMutex`] for [`RefCell`] (single-threaded) and, with the
//! `critical-section` feature, for [`critical_section::Mutex`]. For other mutexes, such as
//! `std::sync::Mutex`, implement it on a newtype:
//!
//! ```
//! # extern crate std;
//! use tca6408a::pins::PortMutex;
//!
//! struct StdMutex<T>(std::sync::Mutex<T>);
//!
//! impl<T> PortMutex for StdMutex<T> {
//!     type Port = T;
//!
//!     fn new(port: T) -> Self {
//!         Self(std::sync::Mutex::new(port))
//!     }
//!
//!     fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
//!         f(&mut self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
//!     }
//!
//!     fn into_inner(self) -> T {
//!         self.0.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner)
//!     }
//! }
//! ```
//!
//! The cache is read once by [`Tca6408a::into_shared`]. Changes to the Output or
//! Configuration registers through [`Shared::with_driver`] bypass it, so pins would write
//! stale values back.
//!
//! ## Direction
//!
//! A pin's direction is a runtime property ([`Pin::make_output`], [`Pin::make_input`]),
//! not a typestate. The chip accepts every operation in either direction: the Input
//! register reports the level of outputs, too, and writing the latch of an input has no
//! effect until the pin becomes an output (datasheet section 8.6.3). A typestate would
//! rule out no invalid state, but would make each conversion consume the pin and hand it
//! back on an I²C error.
//!
//! ## Input reads
//!
//! [`InputPin::is_high`] reads the Input register: one I²C read, which also clears the
//! interrupt for all pins. The level is inverted for pins selected in the Polarity Inversion
//! register.
//!
//! ## No async pins
//!
//! The only digital trait in `embedded-hal-async` is `Wait`. Waiting for a pin change needs
//! the `INT` line wired to the host and an interrupt handler, which is outside this driver,
//! so no async pin handles are provided.

use crate::{Latch, Tca6408a};
use core::cell::RefCell;
use embedded_hal::digital::{self, ErrorKind, ErrorType, InputPin, OutputPin, StatefulOutputPin};
use embedded_hal::i2c::I2c;

/// A mutex that shares the [`Port`] between the pins.
pub trait PortMutex {
    /// The protected state.
    type Port;

    /// Creates the mutex.
    fn new(port: Self::Port) -> Self;

    /// Runs `f` with exclusive access to the state.
    fn lock<R>(&self, f: impl FnOnce(&mut Self::Port) -> R) -> R;

    /// Consumes the mutex and returns the state.
    fn into_inner(self) -> Self::Port;
}

impl<T> PortMutex for RefCell<T> {
    type Port = T;

    fn new(port: T) -> Self {
        RefCell::new(port)
    }

    fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.borrow_mut())
    }

    fn into_inner(self) -> T {
        RefCell::into_inner(self)
    }
}

#[cfg(feature = "critical-section")]
#[cfg_attr(docsrs, doc(cfg(feature = "critical-section")))]
impl<T> PortMutex for critical_section::Mutex<RefCell<T>> {
    type Port = T;

    fn new(port: T) -> Self {
        critical_section::Mutex::new(RefCell::new(port))
    }

    fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        critical_section::with(|cs| f(&mut self.borrow_ref_mut(cs)))
    }

    fn into_inner(self) -> T {
        critical_section::Mutex::into_inner(self).into_inner()
    }
}

/// The state shared by the pins: the driver and the cached registers.
#[derive(Debug)]
pub struct Port<I2C> {
    driver: Tca6408a<I2C>,
    latch: Latch,
}

/// An error of a pin operation: the underlying I²C error.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PinError<E>(pub E);

impl<E: core::fmt::Debug> digital::Error for PinError<E> {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl<E> From<E> for PinError<E> {
    fn from(error: E) -> Self {
        Self(error)
    }
}

impl<I2C> Tca6408a<I2C>
where
    I2C: I2c,
{
    /// Moves the driver into a [`PortMutex`] for use with [`Pin`]s.
    ///
    /// Reads the Output and Configuration registers once to fill the cache.
    #[cfg_attr(docsrs, doc(cfg(feature = "pins")))]
    pub fn into_shared<M>(mut self) -> Result<Shared<M>, I2C::Error>
    where
        M: PortMutex<Port = Port<I2C>>,
    {
        let latch = Latch {
            output: self.read_outputs()?,
            config: self.read_configuration()?,
        };

        Ok(Shared {
            mutex: M::new(Port {
                driver: self,
                latch,
            }),
        })
    }
}

/// The driver, shared between [`Pin`]s through the mutex `M`.
#[derive(Debug)]
pub struct Shared<M> {
    mutex: M,
}

impl<M, I2C> Shared<M>
where
    M: PortMutex<Port = Port<I2C>>,
    I2C: I2c,
{
    /// Hands out the pins `P0` to `P7`.
    ///
    /// Calling it again hands out further handles to the same pins. They stay consistent,
    /// since all handles go through the shared state.
    pub fn split(&self) -> Parts<'_, M> {
        let pin = |bit: u8| Pin {
            shared: &self.mutex,
            mask: 1 << bit,
        };

        Parts {
            p0: pin(0),
            p1: pin(1),
            p2: pin(2),
            p3: pin(3),
            p4: pin(4),
            p5: pin(5),
            p6: pin(6),
            p7: pin(7),
        }
    }

    /// Runs `f` with exclusive access to the driver, for example to set the polarity
    /// inversion or to change several pins with one write.
    ///
    /// Changes to the Output and Configuration registers made here bypass the cache.
    pub fn with_driver<R>(&self, f: impl FnOnce(&mut Tca6408a<I2C>) -> R) -> R {
        self.mutex.lock(|port| f(&mut port.driver))
    }

    /// Consumes the shared state and returns the driver.
    pub fn release(self) -> Tca6408a<I2C> {
        self.mutex.into_inner().driver
    }
}

/// The pins `P0` to `P7`, as returned by [`Shared::split`].
#[derive(Debug)]
#[allow(missing_docs)]
pub struct Parts<'a, M> {
    pub p0: Pin<'a, M>,
    pub p1: Pin<'a, M>,
    pub p2: Pin<'a, M>,
    pub p3: Pin<'a, M>,
    pub p4: Pin<'a, M>,
    pub p5: Pin<'a, M>,
    pub p6: Pin<'a, M>,
    pub p7: Pin<'a, M>,
}

/// A single pin of the expander.
#[derive(Debug)]
pub struct Pin<'a, M> {
    shared: &'a M,
    mask: u8,
}

impl<M, I2C> Pin<'_, M>
where
    M: PortMutex<Port = Port<I2C>>,
    I2C: I2c,
{
    /// The bit of this pin in the port registers: `1 << n` for `Pn`.
    pub fn mask(&self) -> u8 {
        self.mask
    }

    /// Makes the pin an output driving `high`.
    ///
    /// Writes the output latch before the direction, so the pin never drives a stale level.
    pub fn make_output(&mut self, high: bool) -> Result<(), PinError<I2C::Error>> {
        let levels = if high { self.mask } else { 0 };
        self.shared.lock(|port| {
            let target = port.latch.with_outputs(self.mask, levels);
            port.driver.write_latch(&mut port.latch, target)?;
            Ok(())
        })
    }

    /// Makes the pin a high-impedance input.
    pub fn make_input(&mut self) -> Result<(), PinError<I2C::Error>> {
        self.shared.lock(|port| {
            let config = port.latch.config | self.mask;
            port.driver.write_configuration(config)?;
            port.latch.config = config;
            Ok(())
        })
    }

    /// Whether the pin is configured as an output, from the cache.
    pub fn is_output(&self) -> bool {
        self.shared.lock(|port| port.latch.config & self.mask == 0)
    }

    fn set_latch(&mut self, high: bool) -> Result<(), PinError<I2C::Error>> {
        self.shared.lock(|port| {
            let output = if high {
                port.latch.output | self.mask
            } else {
                port.latch.output & !self.mask
            };
            port.driver.write_outputs(output)?;
            port.latch.output = output;
            Ok(())
        })
    }
}

impl<M, I2C> ErrorType for Pin<'_, M>
where
    M: PortMutex<Port = Port<I2C>>,
    I2C: I2c,
{
    type Error = PinError<I2C::Error>;
}

impl<M, I2C> InputPin for Pin<'_, M>
where
    M: PortMutex<Port = Port<I2C>>,
    I2C: I2c,
{
    /// Reads the Input register, which clears the interrupt. Honors polarity inversion.
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        let inputs = self.shared.lock(|port| port.driver.read_inputs())?;
        Ok(inputs & self.mask != 0)
    }

    /// Reads the Input register, which clears the interrupt. Honors polarity inversion.
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_high()?)
    }
}

impl<M, I2C> OutputPin for Pin<'_, M>
where
    M: PortMutex<Port = Port<I2C>>,
    I2C: I2c,
{
    /// Sets the output latch high with one I²C write. Takes effect while the pin is an
    /// output.
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_latch(true)
    }

    /// Sets the output latch low with one I²C write. Takes effect while the pin is an
    /// output.
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_latch(false)
    }
}

impl<M, I2C> StatefulOutputPin for Pin<'_, M>
where
    M: PortMutex<Port = Port<I2C>>,
    I2C: I2c,
{
    /// Whether the output latch is high, from the cache.
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.shared.lock(|port| port.latch.output & self.mask != 0))
    }

    /// Whether the output latch is low, from the cache.
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_set_high()?)
    }
}
