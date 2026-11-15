//! # TI TCA6408A behavioral model
//!
//! A register-level model of the TCA6408A I/O expander, used in two places:
//!
//! - as a fake I²C bus ([`FakeI2c`]) in host tests of the `tca6408a` driver, and
//! - wrapped into a [Wokwi](https://wokwi.com) custom chip (`sim/wokwi-chip`).
//!
//! The model implements, following the datasheet (SCPS192E):
//!
//! - the I²C protocol: a command byte selects the register and stays selected; further
//!   bytes of a read transfer re-read it (section 8.5.2.2, Figure 8-8),
//! - the Input, Output, Polarity Inversion and Configuration registers with their power-up
//!   defaults (Table 8-4), and the pin levels they result in,
//! - the `INT` output: asserted while any input differs from the levels captured by the
//!   last read of the Input register; cleared by that read or when the inputs return to
//!   those levels (section 8.3.3),
//! - the `RESET` input and the power-on reset, which restore the defaults (sections 8.3.4,
//!   8.4.1).
//!
//! ## Not modelled
//!
//! Timing (`tiv`, `tir`, `tpv`, `tW`): changes take effect immediately. Electrical
//! behavior, such as contention between an output and an external driver.
//!
//! ## Assumptions
//!
//! The datasheet leaves these points open; the model behaves as follows:
//!
//! - The command byte is `0x00` (Input register) after reset.
//! - Command bytes other than `0x00`–`0x03` are not acknowledged and leave the selected
//!   register unchanged.
//! - Further data bytes of a write transfer are written to the same register; the
//!   selection does not auto-increment, as for reads.
//! - Polarity inversion applies only to pins configured as inputs ("pins defined as inputs",
//!   section 8.6.3); the Input register reports output pins at their driven level.
//! - Reset captures the current pin levels as the interrupt reference, so `INT` is
//!   released after reset.
//! - While `RESET` is held low, the chip does not acknowledge its address.
//! - The Input register is captured, and `INT` is cleared, when a byte of it is read.
//! - Undriven inputs read high, as with the external pull-ups the datasheet asks for
//!   (Figure 9-1, note D).

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bus;

pub use bus::FakeI2c;

use tca6408a_registers::{
    Configuration, DEFAULT_DEVICE_ADDRESS, OutputPort, PolarityInversion, RegisterAddress as Reg,
};

/// A behavioral model of the TCA6408A.
#[derive(Debug, Clone)]
pub struct Tca6408aModel {
    address: u8,
    output: u8,
    polarity: u8,
    config: u8,
    /// The register selected by the last command byte.
    pointer: Reg,
    /// `true` until the first byte of a write transaction set the command byte.
    expect_command: bool,
    /// The levels applied externally to the pins; only relevant for inputs.
    external: u8,
    /// The pin levels captured by the last read of the Input register.
    captured: u8,
    in_reset: bool,
}

impl Default for Tca6408aModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Tca6408aModel {
    /// Creates a model with `ADDR` tied low ([`DEFAULT_DEVICE_ADDRESS`]), freshly powered on.
    pub fn new() -> Self {
        Self::with_address(DEFAULT_DEVICE_ADDRESS)
    }

    /// Creates a model listening on the specified 7-bit address, freshly powered on.
    ///
    /// The real chip answers on `0x20` or `0x21`, selected by its `ADDR` pin.
    pub fn with_address(address: u8) -> Self {
        let mut model = Self {
            address,
            output: 0,
            polarity: 0,
            config: 0,
            pointer: Reg::INPUT_PORT,
            expect_command: false,
            // Inputs are high-impedance; assume external pull-ups until driven.
            external: 0xFF,
            captured: 0,
            in_reset: false,
        };

        model.reset();
        model
    }

    /// Restores the power-up defaults, as by a power-on reset or a low pulse on `RESET`.
    pub fn reset(&mut self) {
        self.output = OutputPort::new().into();
        self.polarity = PolarityInversion::new().into();
        self.config = Configuration::new().into();
        self.pointer = Reg::INPUT_PORT;
        self.expect_command = false;
        self.captured = self.pins();
    }

    /// Sets the level of the active-low `RESET` pin.
    ///
    /// While the pin is low, the registers stay at their defaults and the chip does not
    /// respond on the bus.
    pub fn set_reset_pin(&mut self, high: bool) {
        // Resetting on release, too, captures the pin levels as they are then.
        if self.in_reset || !high {
            self.reset();
        }

        self.in_reset = !high;
    }

    /// The 7-bit I²C address of the model.
    pub fn address(&self) -> u8 {
        self.address
    }

    /// Reads a register directly, bypassing the bus. Does not clear the interrupt.
    pub fn register(&self, reg: Reg) -> u8 {
        match reg {
            Reg::INPUT_PORT => self.pins() ^ (self.polarity & self.config),
            Reg::OUTPUT_PORT => self.output,
            Reg::POLARITY_INVERSION => self.polarity,
            Reg::CONFIGURATION => self.config,
        }
    }

    /// Applies external levels to the pins, bit `n` to `Pn`. Pins configured as outputs
    /// ignore them.
    pub fn drive(&mut self, levels: u8) {
        self.external = levels;
    }

    /// The pin levels: the output latch for outputs, the external level for inputs.
    pub fn pins(&self) -> u8 {
        (self.output & !self.config) | (self.external & self.config)
    }

    /// The pins configured as outputs, bit `n` for `Pn`.
    pub fn outputs(&self) -> u8 {
        !self.config
    }

    /// Whether the open-drain, active-low `INT` output is asserted (pulled low).
    ///
    /// Only pins configured as inputs cause an interrupt.
    pub fn interrupt(&self) -> bool {
        !self.in_reset && (self.pins() ^ self.captured) & self.config != 0
    }

    /// Starts a transaction addressed to the model. Returns `true` to acknowledge.
    ///
    /// `is_read` selects a read (`true`) or write (`false`) transfer.
    pub fn i2c_start(&mut self, is_read: bool) -> bool {
        self.expect_command = !is_read;
        !self.in_reset
    }

    /// Writes a byte within a write transfer. Returns `true` to acknowledge.
    ///
    /// The first byte is the command byte that selects the register; subsequent bytes are
    /// written to it. Writes to the Input register have no effect.
    pub fn i2c_write(&mut self, byte: u8) -> bool {
        if self.in_reset {
            return false;
        }

        if self.expect_command {
            let Some(reg) = Reg::from_addr(byte) else {
                return false;
            };

            self.pointer = reg;
            self.expect_command = false;
            return true;
        }

        match self.pointer {
            Reg::INPUT_PORT => {}
            Reg::OUTPUT_PORT => self.output = byte,
            Reg::POLARITY_INVERSION => self.polarity = byte,
            Reg::CONFIGURATION => self.config = byte,
        }

        true
    }

    /// Reads the selected register within a read transfer.
    ///
    /// Reading the Input register captures the pin levels and clears the interrupt.
    pub fn i2c_read(&mut self) -> u8 {
        if self.pointer == Reg::INPUT_PORT {
            self.captured = self.pins();
        }

        self.register(self.pointer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(model: &mut Tca6408aModel, bytes: &[u8]) -> bool {
        model.i2c_start(false) && bytes.iter().all(|&b| model.i2c_write(b))
    }

    fn read<const N: usize>(model: &mut Tca6408aModel, reg: u8) -> [u8; N] {
        assert!(write(model, &[reg]));
        assert!(model.i2c_start(true));
        core::array::from_fn(|_| model.i2c_read())
    }

    #[test]
    fn power_up_defaults() {
        let mut model = Tca6408aModel::new();
        assert_eq!(read::<1>(&mut model, 0x01), [0xFF]);
        assert_eq!(read::<1>(&mut model, 0x02), [0x00]);
        assert_eq!(read::<1>(&mut model, 0x03), [0xFF]);
        assert_eq!(read::<1>(&mut model, 0x00), [0xFF]);
        assert!(!model.interrupt());
    }

    #[test]
    fn command_byte_persists_and_does_not_increment() {
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x03, 0x0F]));
        assert!(model.i2c_start(true));
        assert_eq!([model.i2c_read(), model.i2c_read()], [0x0F, 0x0F]);

        assert!(write(&mut model, &[0x01, 0x12, 0x34]));
        assert_eq!(model.register(Reg::OUTPUT_PORT), 0x34);
        assert_eq!(model.register(Reg::POLARITY_INVERSION), 0x00);
    }

    #[test]
    fn invalid_command_nacks() {
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x03]));
        assert!(!write(&mut model, &[0x04]));
        assert!(model.i2c_start(true));
        assert_eq!(model.i2c_read(), 0xFF);
    }

    #[test]
    fn input_register_ignores_writes() {
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x00, 0x00]));
        assert_eq!(model.register(Reg::INPUT_PORT), 0xFF);
    }

    #[test]
    fn outputs_follow_latch_and_read_back() {
        let mut model = Tca6408aModel::new();
        model.drive(0x00);
        assert!(write(&mut model, &[0x01, 0x80]));
        assert!(write(&mut model, &[0x03, 0x7F]));
        assert_eq!(model.pins(), 0x80);
        assert_eq!(model.outputs(), 0x80);
        assert_eq!(read::<1>(&mut model, 0x00), [0x80]);
    }

    #[test]
    fn polarity_inverts_inputs_only() {
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x02, 0x81]));
        assert!(write(&mut model, &[0x03, 0x7F]));
        assert_eq!(read::<1>(&mut model, 0x00), [0xFE]);
    }

    #[test]
    fn interrupt_on_change_cleared_by_read() {
        let mut model = Tca6408aModel::new();
        model.drive(0xFE);
        assert!(model.interrupt());
        assert_eq!(read::<1>(&mut model, 0x00), [0xFE]);
        assert!(!model.interrupt());
    }

    #[test]
    fn interrupt_cleared_by_returning_input() {
        let mut model = Tca6408aModel::new();
        model.drive(0xFE);
        assert!(model.interrupt());
        model.drive(0xFF);
        assert!(!model.interrupt());
    }

    #[test]
    fn reading_other_registers_keeps_interrupt() {
        let mut model = Tca6408aModel::new();
        model.drive(0xFE);
        let _ = read::<1>(&mut model, 0x03);
        assert!(model.interrupt());
    }

    #[test]
    fn outputs_do_not_interrupt() {
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x03, 0x7F]));
        model.drive(0x7F);
        assert!(!model.interrupt());
        assert!(write(&mut model, &[0x01, 0x7F]));
        assert!(!model.interrupt());
    }

    #[test]
    fn output_to_input_may_interrupt() {
        // Datasheet 8.3.3: a pin switched back to input interrupts if its level differs
        // from the Input register contents.
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x01, 0x7F]));
        assert!(write(&mut model, &[0x03, 0x7F]));
        let _ = read::<1>(&mut model, 0x00);
        assert!(write(&mut model, &[0x03, 0xFF]));
        assert!(model.interrupt());
    }

    #[test]
    fn reset_pin_restores_defaults_and_blocks_bus() {
        let mut model = Tca6408aModel::new();
        assert!(write(&mut model, &[0x03, 0x00]));
        assert!(write(&mut model, &[0x01, 0x00]));
        model.set_reset_pin(false);
        assert!(!model.i2c_start(false));
        assert!(!model.i2c_write(0x01));
        model.set_reset_pin(true);
        assert_eq!(model.register(Reg::CONFIGURATION), 0xFF);
        assert_eq!(model.register(Reg::OUTPUT_PORT), 0xFF);
        // The command byte returns to the Input register.
        assert!(model.i2c_start(true));
        assert_eq!(model.i2c_read(), 0xFF);
    }
}
