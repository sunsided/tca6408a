//! Device registers.
//!
//! Register addresses (command bytes), bit names and power-up defaults follow the
//! datasheet, sections 8.6.2 and 8.6.3.

use bitfield_struct::bitfield;

/// Register addresses, as sent in the command byte (datasheet Table 8-4).
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum RegisterAddress {
    /// See [`InputPort`]. Read-only.
    INPUT_PORT = 0x00,
    /// See [`OutputPort`]. Read-write.
    OUTPUT_PORT = 0x01,
    /// See [`PolarityInversion`]. Read-write.
    POLARITY_INVERSION = 0x02,
    /// See [`Configuration`]. Read-write.
    CONFIGURATION = 0x03,
}

impl RegisterAddress {
    /// Returns the address of a register.
    pub const fn addr(&self) -> u8 {
        *self as u8
    }

    /// Converts a command byte into a register address, if it names one.
    pub const fn from_addr(addr: u8) -> Option<Self> {
        match addr {
            0x00 => Some(Self::INPUT_PORT),
            0x01 => Some(Self::OUTPUT_PORT),
            0x02 => Some(Self::POLARITY_INVERSION),
            0x03 => Some(Self::CONFIGURATION),
            _ => None,
        }
    }
}

impl From<RegisterAddress> for u8 {
    fn from(value: RegisterAddress) -> Self {
        value.addr()
    }
}

/// Declares a register with one bit per port pin, `P0` in bit 0.
macro_rules! port_register {
    (
        $(#[$meta:meta])* $type:ident, $addr:ident, $access:ident, $writable:ident, $default:expr,
        $($(#[$field_meta:meta])* $field:ident),+
    ) => {
        $(#[$meta])*
        #[bitfield(u8, order = Lsb)]
        #[derive(PartialEq, Eq)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct $type {
            $(
                $(#[$field_meta])*
                #[bits(1, access = $access, default = $default)]
                pub $field: bool,
            )+
        }

        $writable!($type, RegisterAddress::$addr);
    };
}

port_register!(
    /// [`INPUT_PORT`](RegisterAddress::INPUT_PORT) (00h)
    ///
    /// The incoming logic levels of the pins, whether a pin is configured as an input or an
    /// output, inverted for pins selected in [`PolarityInversion`]. Writes have no effect.
    /// The power-up value depends on the externally applied levels.
    ///
    /// Reading this register clears the interrupt (datasheet section 8.3.3).
    InputPort, INPUT_PORT, RO, readable_register, false,
    /// The level of `P0` (`I-0`).
    p0,
    /// The level of `P1` (`I-1`).
    p1,
    /// The level of `P2` (`I-2`).
    p2,
    /// The level of `P3` (`I-3`).
    p3,
    /// The level of `P4` (`I-4`).
    p4,
    /// The level of `P5` (`I-5`).
    p5,
    /// The level of `P6` (`I-6`).
    p6,
    /// The level of `P7` (`I-7`).
    p7
);

port_register!(
    /// [`OUTPUT_PORT`](RegisterAddress::OUTPUT_PORT) (01h)
    ///
    /// The outgoing levels of pins configured as outputs. Bits of pins configured as inputs
    /// have no effect. Reads return the output latch, not the pin level.
    /// Power-up default: `0xFF`.
    OutputPort, OUTPUT_PORT, RW, writable_register, true,
    /// The output level of `P0` (`O-0`).
    p0,
    /// The output level of `P1` (`O-1`).
    p1,
    /// The output level of `P2` (`O-2`).
    p2,
    /// The output level of `P3` (`O-3`).
    p3,
    /// The output level of `P4` (`O-4`).
    p4,
    /// The output level of `P5` (`O-5`).
    p5,
    /// The output level of `P6` (`O-6`).
    p6,
    /// The output level of `P7` (`O-7`).
    p7
);

port_register!(
    /// [`POLARITY_INVERSION`](RegisterAddress::POLARITY_INVERSION) (02h)
    ///
    /// A set bit inverts the polarity of the pin's bit in [`InputPort`], for pins
    /// configured as inputs. Power-up default: `0x00`.
    PolarityInversion, POLARITY_INVERSION, RW, writable_register, false,
    /// Inverts `P0` (`N-0`).
    p0,
    /// Inverts `P1` (`N-1`).
    p1,
    /// Inverts `P2` (`N-2`).
    p2,
    /// Inverts `P3` (`N-3`).
    p3,
    /// Inverts `P4` (`N-4`).
    p4,
    /// Inverts `P5` (`N-5`).
    p5,
    /// Inverts `P6` (`N-6`).
    p6,
    /// Inverts `P7` (`N-7`).
    p7
);

port_register!(
    /// [`CONFIGURATION`](RegisterAddress::CONFIGURATION) (03h)
    ///
    /// The direction of the pins: a set bit makes the pin a high-impedance input, a
    /// cleared bit makes it an output. Power-up default: `0xFF` (all inputs).
    Configuration, CONFIGURATION, RW, writable_register, true,
    /// `P0` is an input (`C-0`).
    p0,
    /// `P1` is an input (`C-1`).
    p1,
    /// `P2` is an input (`C-2`).
    p2,
    /// `P3` is an input (`C-3`).
    p3,
    /// `P4` is an input (`C-4`).
    p4,
    /// `P5` is an input (`C-5`).
    p5,
    /// `P6` is an input (`C-6`).
    p6,
    /// `P7` is an input (`C-7`).
    p7
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn power_up_defaults() {
        assert_eq!(u8::from(OutputPort::new()), 0xFF);
        assert_eq!(u8::from(PolarityInversion::new()), 0x00);
        assert_eq!(u8::from(Configuration::new()), 0xFF);
    }

    #[test]
    fn p0_is_bit_0() {
        assert_eq!(u8::from(PolarityInversion::new().with_p0(true)), 0x01);
        assert_eq!(u8::from(Configuration::new().with_p7(false)), 0x7F);
        assert!(InputPort::from(0x80).p7());
        assert!(!InputPort::from(0x80).p6());
    }

    #[test]
    fn register_addresses() {
        assert_eq!(*InputPort::REGISTER_ADDRESS, 0x00);
        assert_eq!(*OutputPort::REGISTER_ADDRESS, 0x01);
        assert_eq!(*PolarityInversion::REGISTER_ADDRESS, 0x02);
        assert_eq!(*Configuration::REGISTER_ADDRESS, 0x03);
        assert_eq!(*InputPort::DEFAULT_DEVICE_ADDRESS, 0x20);
    }

    #[test]
    fn command_byte_roundtrip() {
        for reg in [
            super::RegisterAddress::INPUT_PORT,
            super::RegisterAddress::OUTPUT_PORT,
            super::RegisterAddress::POLARITY_INVERSION,
            super::RegisterAddress::CONFIGURATION,
        ] {
            assert_eq!(super::RegisterAddress::from_addr(reg.addr()), Some(reg));
        }

        assert_eq!(super::RegisterAddress::from_addr(0x04), None);
    }
}
