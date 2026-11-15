//! # TI TCA6408A I/O expander driver
//!
//! > A platform-agnostic driver to interface with the TI TCA6408A 8-bit I²C I/O expander.
//!
//! This driver was built using [`embedded-hal`] 1.0 and [`tca6408a-registers`]. Enable the
//! `async` feature for an equivalent driver on [`embedded-hal-async`] in the `asynch` module.
//!
//! ## Pins
//!
//! All methods take and return whole ports: bit `n` belongs to pin `Pn`. Methods that
//! change some pins take a mask of the pins to change and leave the others as they are.
//!
//! After power-on or [`reset`], all pins are inputs and the output latch is `0xFF`. To make
//! a pin an output without a glitch, use [`Tca6408a::make_outputs`]: it sets the output
//! latch before switching the direction, so the pin starts driving the requested level.
//!
//! ## Interrupt
//!
//! The open-drain, active-low `INT` output is asserted when a pin configured as an input
//! changes. Reading the inputs ([`Tca6408a::read_inputs`]) clears it, as does the input
//! returning to its previous level.
//!
//! [`embedded-hal`]: https://crates.io/crates/embedded-hal
//! [`embedded-hal-async`]: https://crates.io/crates/embedded-hal-async
//! [`tca6408a-registers`]: https://crates.io/crates/tca6408a-registers

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![forbid(unsafe_code)]
// Enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub mod asynch;

pub use tca6408a_registers as registers;
pub use tca6408a_registers::{ALTERNATE_DEVICE_ADDRESS, DEFAULT_DEVICE_ADDRESS};

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::i2c::I2c;
use tca6408a_registers::prelude::*;
use tca6408a_registers::{Configuration, InputPort, OutputPort, PolarityInversion};

/// How long `RESET` is held low during [`reset`], in nanoseconds.
///
/// The datasheet requires at least 4 ns (`tW`) and resets the pins within 600 ns
/// (`tRESET`); 1 µs covers both. The chip is ready right after `RESET` rises (`tREC` = 0).
pub const RESET_PULSE_NS: u32 = 1_000;

/// Resets the chip via its active-low `RESET` pin.
///
/// Restores the power-up defaults: all pins inputs, output latch `0xFF`, no inversion.
pub fn reset<P, D>(rst: &mut P, delay: &mut D) -> Result<(), P::Error>
where
    P: OutputPin,
    D: DelayNs,
{
    rst.set_low()?;
    delay.delay_ns(RESET_PULSE_NS);
    rst.set_high()
}

/// The Output and Configuration registers, as last written.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Latch {
    pub(crate) output: u8,
    pub(crate) config: u8,
}

impl Latch {
    /// Makes the masked pins outputs driving the matching bits of `levels`.
    pub(crate) const fn with_outputs(self, pins: u8, levels: u8) -> Self {
        Self {
            output: self.output & !pins | levels & pins,
            config: self.config & !pins,
        }
    }
}

/// TCA6408A driver.
#[derive(Debug)]
pub struct Tca6408a<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> Tca6408a<I2C>
where
    I2C: I2c,
{
    /// Creates a driver for a chip with `ADDR` tied low ([`DEFAULT_DEVICE_ADDRESS`]).
    /// Performs no bus access.
    pub fn new(i2c: I2C) -> Self {
        Self::with_address(i2c, DEFAULT_DEVICE_ADDRESS)
    }

    /// Creates a driver for a chip at the specified 7-bit address, such as
    /// [`ALTERNATE_DEVICE_ADDRESS`] with `ADDR` tied high. Performs no bus access.
    pub fn with_address(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    /// Consumes the driver and returns the I²C bus.
    pub fn release(self) -> I2C {
        self.i2c
    }

    /// Reads the pin levels, inverted where enabled via
    /// [`write_polarity_inversion`](Self::write_polarity_inversion). Clears the interrupt.
    ///
    /// Pins configured as outputs report their actual level.
    pub fn read_inputs(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<InputPort>()?.into())
    }

    /// Reads the output latch. Returns the latched levels, not the pin levels.
    pub fn read_outputs(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<OutputPort>()?.into())
    }

    /// Writes the output latch. Only pins configured as outputs drive its levels.
    pub fn write_outputs(&mut self, levels: u8) -> Result<(), I2C::Error> {
        self.write_register(OutputPort::from(levels))
    }

    /// Sets the output latch of the masked pins high.
    pub fn set_high(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: OutputPort| OutputPort::from(u8::from(r) | pins))
    }

    /// Sets the output latch of the masked pins low.
    pub fn set_low(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: OutputPort| OutputPort::from(u8::from(r) & !pins))
    }

    /// Reads which pins have their input polarity inverted.
    pub fn read_polarity_inversion(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<PolarityInversion>()?.into())
    }

    /// Selects the pins whose input polarity is inverted, for example for active-low
    /// buttons.
    pub fn write_polarity_inversion(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.write_register(PolarityInversion::from(pins))
    }

    /// Reads the pin directions: a set bit is an input, a cleared bit an output.
    pub fn read_configuration(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<Configuration>()?.into())
    }

    /// Writes the pin directions: a set bit makes the pin an input, a cleared bit an output.
    ///
    /// Prefer [`make_outputs`](Self::make_outputs) to turn pins into outputs.
    pub fn write_configuration(&mut self, inputs: u8) -> Result<(), I2C::Error> {
        self.write_register(Configuration::from(inputs))
    }

    /// Makes the masked pins high-impedance inputs.
    pub fn make_inputs(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: Configuration| Configuration::from(u8::from(r) | pins))
    }

    /// Makes the masked pins outputs driving the matching bits of `levels`.
    ///
    /// Writes the output latch before the direction, so the pins never drive a stale
    /// latch value.
    pub fn make_outputs(&mut self, pins: u8, levels: u8) -> Result<(), I2C::Error> {
        let mut latch = Latch {
            output: self.read_outputs()?,
            config: self.read_configuration()?,
        };
        let target = latch.with_outputs(pins, levels);
        self.write_latch(&mut latch, target)
    }

    /// Writes the output latch, then the direction: the order that keeps pins turning into
    /// outputs glitch-free. Updates `cache` after each write that succeeded.
    pub(crate) fn write_latch(
        &mut self,
        cache: &mut Latch,
        target: Latch,
    ) -> Result<(), I2C::Error> {
        self.write_outputs(target.output)?;
        cache.output = target.output;
        self.write_configuration(target.config)?;
        cache.config = target.config;
        Ok(())
    }

    /// Reads a single register.
    pub fn read_register<R>(&mut self) -> Result<R, I2C::Error>
    where
        R: ReadableRegister,
    {
        let mut buffer = [0];
        self.i2c
            .write_read(self.address, &[*R::REGISTER_ADDRESS], &mut buffer)?;
        Ok(R::from_bits(buffer[0]))
    }

    /// Writes a single register.
    pub fn write_register<R>(&mut self, register: R) -> Result<(), I2C::Error>
    where
        R: WritableRegister,
    {
        self.i2c
            .write(self.address, &[*R::REGISTER_ADDRESS, register.to_bits()])
    }

    /// Reads, modifies and writes back a single register.
    pub fn modify_register<R, F>(&mut self, f: F) -> Result<(), I2C::Error>
    where
        R: ReadableRegister + WritableRegister,
        F: FnOnce(R) -> R,
    {
        let register = self.read_register::<R>()?;
        self.write_register(f(register))
    }
}
