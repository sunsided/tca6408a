//! # TI TCA6408A I/O Expander Registers
//!
//! This crate provides a typed map of the TCA6408A's I²C registers, following the
//! TCA6408A datasheet (SCPS192E, revised January 2023).
//!
//! The chip has four 8-bit registers, selected by the command byte that follows the
//! address byte of a write transfer. Bit `n` of each register belongs to port pin `Pn`.

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![forbid(unsafe_code)]
// bitfield-struct generates `with_*_checked` setters that return `Result<_, ()>`.
#![allow(clippy::result_unit_err)]
// Enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined.
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Exports commonly used traits.
pub mod prelude {
    pub use crate::{ReadableRegister, Register, WritableRegister};
    pub use hardware_registers::i2c::*;
    pub use hardware_registers::register_address::RegisterAddress8;
    pub use hardware_registers::sizes::R1;
    pub use hardware_registers::{FromBits, HardwareRegister, ToBits, WritableHardwareRegister};
}

/// Implements the address and size information shared by all registers.
macro_rules! register {
    ($type:ident, $addr:expr) => {
        impl $crate::Register for $type {}
        impl $crate::prelude::HardwareRegister<$crate::prelude::R1> for $type {}

        impl
            $crate::prelude::I2CRegister<
                $crate::prelude::DeviceAddress7,
                $crate::prelude::RegisterAddress8,
                $crate::prelude::R1,
            > for $type
        {
            type Backing = u8;

            const DEFAULT_DEVICE_ADDRESS: $crate::prelude::DeviceAddress7 =
                $crate::prelude::DeviceAddress7::new($crate::DEFAULT_DEVICE_ADDRESS);
            const REGISTER_ADDRESS: $crate::prelude::RegisterAddress8 =
                $crate::prelude::RegisterAddress8::new(($addr).addr());
        }
    };
}

/// Implements decoding, which makes a register readable.
macro_rules! read_access {
    ($type:ident) => {
        impl $crate::prelude::FromBits<u8> for $type {
            #[inline]
            fn from_bits(value: u8) -> Self {
                value.into()
            }

            #[inline]
            fn from_bits_ref(value: &u8) -> Self {
                (*value).into()
            }
        }

        impl $crate::ReadableRegister for $type {}
    };
}

/// Implements encoding, which makes a register writable.
macro_rules! write_access {
    ($type:ident) => {
        impl $crate::prelude::ToBits for $type {
            type Target = u8;

            #[inline]
            fn to_bits(&self) -> Self::Target {
                (*self).into()
            }
        }

        impl $crate::prelude::WritableHardwareRegister<$crate::prelude::R1> for $type {}
        impl $crate::WritableRegister for $type {}
    };
}

macro_rules! readable_register {
    ($type:ident, $addr:expr) => {
        register!($type, $addr);
        read_access!($type);
    };
}

macro_rules! writable_register {
    ($type:ident, $addr:expr) => {
        register!($type, $addr);
        read_access!($type);
        write_access!($type);
    };
}

mod registers;

pub use registers::*;

/// The I²C bus address with the `ADDR` pin tied low (datasheet Table 8-3).
pub const DEFAULT_DEVICE_ADDRESS: u8 = 0x20;

/// The I²C bus address with the `ADDR` pin tied high (datasheet Table 8-3).
pub const ALTERNATE_DEVICE_ADDRESS: u8 = 0x21;

/// A device register.
pub trait Register: prelude::I2CRegister8<prelude::DeviceAddress7> {}

/// A readable device register.
pub trait ReadableRegister:
    Register + prelude::ReadableI2CRegister8<prelude::DeviceAddress7> + prelude::FromBits<u8>
{
}

/// A writable device register.
pub trait WritableRegister:
    Register + prelude::WritableI2CRegister8<prelude::DeviceAddress7>
{
}
