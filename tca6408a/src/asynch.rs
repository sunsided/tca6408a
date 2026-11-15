//! Async driver on `embedded-hal-async`.
//!
//! Mirrors the blocking [`Tca6408a`](crate::Tca6408a) and [`reset`](crate::reset).

use crate::{DEFAULT_DEVICE_ADDRESS, RESET_PULSE_NS};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;
use tca6408a_registers::prelude::*;
use tca6408a_registers::{Configuration, InputPort, OutputPort, PolarityInversion};

/// Resets the chip via its active-low `RESET` pin.
///
/// Restores the power-up defaults: all pins inputs, output latch `0xFF`, no inversion.
pub async fn reset<P, D>(rst: &mut P, delay: &mut D) -> Result<(), P::Error>
where
    P: OutputPin,
    D: DelayNs,
{
    rst.set_low()?;
    delay.delay_ns(RESET_PULSE_NS).await;
    rst.set_high()
}

/// Async TCA6408A driver.
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
    /// [`ALTERNATE_DEVICE_ADDRESS`](crate::ALTERNATE_DEVICE_ADDRESS) with `ADDR` tied high.
    /// Performs no bus access.
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
    pub async fn read_inputs(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<InputPort>().await?.into())
    }

    /// Reads the output latch. Returns the latched levels, not the pin levels.
    pub async fn read_outputs(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<OutputPort>().await?.into())
    }

    /// Writes the output latch. Only pins configured as outputs drive its levels.
    pub async fn write_outputs(&mut self, levels: u8) -> Result<(), I2C::Error> {
        self.write_register(OutputPort::from(levels)).await
    }

    /// Sets the output latch of the masked pins high.
    pub async fn set_high(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: OutputPort| OutputPort::from(u8::from(r) | pins))
            .await
    }

    /// Sets the output latch of the masked pins low.
    pub async fn set_low(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: OutputPort| OutputPort::from(u8::from(r) & !pins))
            .await
    }

    /// Reads which pins have their input polarity inverted.
    pub async fn read_polarity_inversion(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<PolarityInversion>().await?.into())
    }

    /// Selects the pins whose input polarity is inverted, for example for active-low
    /// buttons.
    pub async fn write_polarity_inversion(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.write_register(PolarityInversion::from(pins)).await
    }

    /// Reads the pin directions: a set bit is an input, a cleared bit an output.
    pub async fn read_configuration(&mut self) -> Result<u8, I2C::Error> {
        Ok(self.read_register::<Configuration>().await?.into())
    }

    /// Writes the pin directions: a set bit makes the pin an input, a cleared bit an output.
    ///
    /// Prefer [`make_outputs`](Self::make_outputs) to turn pins into outputs.
    pub async fn write_configuration(&mut self, inputs: u8) -> Result<(), I2C::Error> {
        self.write_register(Configuration::from(inputs)).await
    }

    /// Makes the masked pins high-impedance inputs.
    pub async fn make_inputs(&mut self, pins: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: Configuration| Configuration::from(u8::from(r) | pins))
            .await
    }

    /// Makes the masked pins outputs driving the matching bits of `levels`.
    ///
    /// Writes the output latch before the direction, so the pins never drive a stale
    /// latch value.
    pub async fn make_outputs(&mut self, pins: u8, levels: u8) -> Result<(), I2C::Error> {
        self.modify_register(|r: OutputPort| OutputPort::from(u8::from(r) & !pins | levels & pins))
            .await?;
        self.modify_register(|r: Configuration| Configuration::from(u8::from(r) & !pins))
            .await
    }

    /// Reads a single register.
    pub async fn read_register<R>(&mut self) -> Result<R, I2C::Error>
    where
        R: ReadableRegister,
    {
        let mut buffer = [0];
        self.i2c
            .write_read(self.address, &[*R::REGISTER_ADDRESS], &mut buffer)
            .await?;
        Ok(R::from_bits(buffer[0]))
    }

    /// Writes a single register.
    pub async fn write_register<R>(&mut self, register: R) -> Result<(), I2C::Error>
    where
        R: WritableRegister,
    {
        self.i2c
            .write(self.address, &[*R::REGISTER_ADDRESS, register.to_bits()])
            .await
    }

    /// Reads, modifies and writes back a single register.
    pub async fn modify_register<R, F>(&mut self, f: F) -> Result<(), I2C::Error>
    where
        R: ReadableRegister + WritableRegister,
        F: FnOnce(R) -> R,
    {
        let register = self.read_register::<R>().await?;
        self.write_register(f(register)).await
    }
}
