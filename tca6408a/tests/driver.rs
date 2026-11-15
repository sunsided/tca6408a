//! Driver tests against the behavioral model.

use core::cell::RefCell;
use embedded_hal::i2c::{ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation};
use tca6408a::registers::{Configuration, RegisterAddress as Reg};
use tca6408a::{ALTERNATE_DEVICE_ADDRESS, Tca6408a};
use tca6408a_sim::{FakeI2c, Tca6408aModel};

/// Records the bytes of every write to the bus before forwarding it.
struct Recorder<'a> {
    bus: FakeI2c<'a>,
    writes: Vec<Vec<u8>>,
}

impl ErrorType for Recorder<'_> {
    type Error = ErrorKind;
}

impl I2c for Recorder<'_> {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        for op in operations.iter() {
            if let Operation::Write(bytes) = op {
                self.writes.push(bytes.to_vec());
            }
        }

        self.bus.transaction(address, operations)
    }
}

#[test]
fn power_up_defaults() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    assert_eq!(io.read_outputs().unwrap(), 0xFF);
    assert_eq!(io.read_polarity_inversion().unwrap(), 0x00);
    assert_eq!(io.read_configuration().unwrap(), 0xFF);
    assert_eq!(io.read_inputs().unwrap(), 0xFF);
}

#[test]
fn make_outputs_writes_latch_before_direction() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(Recorder {
        bus: FakeI2c::new(&model),
        writes: Vec::new(),
    });

    io.write_outputs(0x00).unwrap();
    io.make_outputs(0x80, 0xFF).unwrap();

    let writes: Vec<_> = io
        .release()
        .writes
        .into_iter()
        .filter(|w| w.len() == 2)
        .collect();
    assert_eq!(
        writes,
        [vec![0x01, 0x00], vec![0x01, 0x80], vec![0x03, 0x7F]]
    );
    assert_eq!(model.borrow().pins() & 0x80, 0x80);
}

#[test]
fn kode_dot_panel_supply() {
    // P7 drives VCI_EN high; P0-P6 stay inputs.
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    io.make_outputs(0x80, 0x80).unwrap();
    assert_eq!(model.borrow().register(Reg::CONFIGURATION), 0x7F);
    assert_eq!(model.borrow().register(Reg::OUTPUT_PORT), 0xFF);
    assert_eq!(model.borrow().outputs(), 0x80);

    io.set_low(0x80).unwrap();
    assert_eq!(model.borrow().pins() & 0x80, 0x00);
    io.set_high(0x80).unwrap();
    assert_eq!(model.borrow().pins() & 0x80, 0x80);

    io.make_inputs(0x80).unwrap();
    assert_eq!(io.read_configuration().unwrap(), 0xFF);
}

#[test]
fn button_press_interrupts_until_read() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    io.make_outputs(0x80, 0x80).unwrap();

    model.borrow_mut().drive(0xFE);
    assert!(model.borrow().interrupt());
    assert_eq!(io.read_inputs().unwrap(), 0xFE);
    assert!(!model.borrow().interrupt());

    model.borrow_mut().drive(0xFF);
    assert!(model.borrow().interrupt());
    assert_eq!(io.read_inputs().unwrap(), 0xFF);
    assert!(!model.borrow().interrupt());
}

#[test]
fn polarity_inversion() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    io.write_polarity_inversion(0x1F).unwrap();
    model.borrow_mut().drive(0xFE);
    assert_eq!(io.read_inputs().unwrap(), 0xE1);
}

#[test]
fn modify_register_preserves_other_bits() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    io.modify_register(|r: Configuration| r.with_p0(false))
        .unwrap();
    io.modify_register(|r: Configuration| r.with_p3(false))
        .unwrap();
    assert_eq!(io.read_configuration().unwrap(), 0xF6);
}

#[test]
fn reset_pin_restores_defaults() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    io.write_configuration(0x00).unwrap();

    model.borrow_mut().set_reset_pin(false);
    assert_eq!(
        io.read_configuration(),
        Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address))
    );
    model.borrow_mut().set_reset_pin(true);
    assert_eq!(io.read_configuration().unwrap(), 0xFF);
}

#[test]
fn addresses() {
    let model = RefCell::new(Tca6408aModel::with_address(ALTERNATE_DEVICE_ADDRESS));
    let mut io = Tca6408a::new(FakeI2c::new(&model));
    assert!(io.read_inputs().is_err());

    let mut io = Tca6408a::with_address(io.release(), ALTERNATE_DEVICE_ADDRESS);
    assert_eq!(io.read_inputs().unwrap(), 0xFF);
}

#[test]
fn reset_timing() {
    use embedded_hal::delay::DelayNs;
    use embedded_hal::digital::{ErrorType, OutputPin};

    struct Pin<'a>(&'a RefCell<Vec<i64>>);
    struct Delay<'a>(&'a RefCell<Vec<i64>>);

    impl ErrorType for Pin<'_> {
        type Error = core::convert::Infallible;
    }

    impl OutputPin for Pin<'_> {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.0.borrow_mut().push(-1);
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.0.borrow_mut().push(-2);
            Ok(())
        }
    }

    impl DelayNs for Delay<'_> {
        fn delay_ns(&mut self, ns: u32) {
            self.0.borrow_mut().push(i64::from(ns));
        }
    }

    // Pin changes are logged as -1 (low) and -2 (high), delays as their duration in ns.
    let log = RefCell::new(Vec::new());
    tca6408a::reset(&mut Pin(&log), &mut Delay(&log)).unwrap();
    let log = log.into_inner();
    assert_eq!((log[0], log[log.len() - 1]), (-1, -2));
    // tW >= 4 ns and tRESET <= 600 ns.
    assert!(log[1..log.len() - 1].iter().sum::<i64>() >= 600);
}

#[cfg(feature = "async")]
mod asynch {
    use super::*;
    use embassy_futures::block_on;

    #[test]
    fn mirrors_blocking_driver() {
        let model = RefCell::new(Tca6408aModel::new());
        let mut io = tca6408a::asynch::Tca6408a::new(FakeI2c::new(&model));
        block_on(async {
            io.make_outputs(0x80, 0x80).await.unwrap();
            assert_eq!(io.read_configuration().await.unwrap(), 0x7F);
            assert_eq!(io.read_outputs().await.unwrap(), 0xFF);

            io.set_low(0x80).await.unwrap();
            assert_eq!(io.read_outputs().await.unwrap(), 0x7F);
            io.set_high(0x80).await.unwrap();
            io.write_outputs(0xC0).await.unwrap();
            assert_eq!(io.read_outputs().await.unwrap(), 0xC0);

            io.write_polarity_inversion(0x01).await.unwrap();
            assert_eq!(io.read_polarity_inversion().await.unwrap(), 0x01);
            model.borrow_mut().drive(0x7E);
            assert!(model.borrow().interrupt());
            assert_eq!(io.read_inputs().await.unwrap(), 0xFF);
            assert!(!model.borrow().interrupt());

            io.make_inputs(0x80).await.unwrap();
            io.write_configuration(0xF0).await.unwrap();
            io.modify_register(|r: Configuration| r.with_p7(false))
                .await
                .unwrap();
            assert_eq!(io.read_configuration().await.unwrap(), 0x70);
        });

        let mut io = tca6408a::asynch::Tca6408a::with_address(io.release(), 0x21);
        assert!(block_on(io.read_inputs()).is_err());
    }

    #[test]
    fn reset_timing() {
        use embedded_hal::digital::{ErrorType, OutputPin};
        use embedded_hal_async::delay::DelayNs;

        struct Pin<'a>(&'a RefCell<Vec<i64>>);
        struct Delay<'a>(&'a RefCell<Vec<i64>>);

        impl ErrorType for Pin<'_> {
            type Error = core::convert::Infallible;
        }

        impl OutputPin for Pin<'_> {
            fn set_low(&mut self) -> Result<(), Self::Error> {
                self.0.borrow_mut().push(-1);
                Ok(())
            }

            fn set_high(&mut self) -> Result<(), Self::Error> {
                self.0.borrow_mut().push(-2);
                Ok(())
            }
        }

        impl DelayNs for Delay<'_> {
            async fn delay_ns(&mut self, ns: u32) {
                self.0.borrow_mut().push(i64::from(ns));
            }
        }

        let log = RefCell::new(Vec::new());
        block_on(tca6408a::asynch::reset(&mut Pin(&log), &mut Delay(&log))).unwrap();
        let log = log.into_inner();
        assert_eq!((log[0], log[log.len() - 1]), (-1, -2));
        assert!(log[1..log.len() - 1].iter().sum::<i64>() >= 600);
    }
}
