//! [`fake_bus`] adapter for the [`Tca6408aModel`].

use crate::Tca6408aModel;
use core::cell::RefCell;
use fake_bus::I2cTarget;

/// An I²C bus with a single [`Tca6408aModel`] attached.
///
/// The model is shared through a [`RefCell`] so that tests can drive pins while a driver
/// owns the bus. For a bus shared with other models, use [`fake_bus::FakeI2c`] with an
/// array of targets.
pub type FakeI2c<'a> = fake_bus::FakeI2c<&'a RefCell<Tca6408aModel>>;

impl I2cTarget for Tca6408aModel {
    fn address(&self) -> u8 {
        Tca6408aModel::address(self)
    }

    fn start(&mut self, is_read: bool) -> bool {
        self.i2c_start(is_read)
    }

    fn write(&mut self, byte: u8) -> bool {
        self.i2c_write(byte)
    }

    fn read(&mut self) -> u8 {
        self.i2c_read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake_bus::embedded_hal::i2c::{ErrorKind, I2c, NoAcknowledgeSource};

    #[test]
    fn write_read() {
        let model = RefCell::new(Tca6408aModel::new());
        let mut bus = FakeI2c::new(&model);
        let mut config = [0];
        bus.write_read(0x20, &[0x03], &mut config).unwrap();
        assert_eq!(config, [0xFF]);
    }

    #[test]
    fn wrong_address_nacks() {
        let model = RefCell::new(Tca6408aModel::new());
        let mut bus = FakeI2c::new(&model);
        assert_eq!(
            bus.write(0x21, &[0x00]),
            Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address))
        );
    }

    #[test]
    fn invalid_command_nacks_data() {
        let model = RefCell::new(Tca6408aModel::new());
        let mut bus = FakeI2c::new(&model);
        assert_eq!(
            bus.write(0x20, &[0x04]),
            Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data))
        );
    }

    #[test]
    fn async_write_read() {
        let model = RefCell::new(Tca6408aModel::new());
        let mut bus = FakeI2c::new(&model);
        let mut output = [0];
        embassy_futures::block_on(fake_bus::embedded_hal_async::i2c::I2c::write_read(
            &mut bus,
            0x20,
            &[0x01],
            &mut output,
        ))
        .unwrap();
        assert_eq!(output, [0xFF]);
    }
}
