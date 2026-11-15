//! Pin handle tests against the behavioral model.
#![cfg(feature = "pins")]

use core::cell::RefCell;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use embedded_hal::i2c::{ErrorKind, ErrorType, I2c, Operation};
use tca6408a::Tca6408a;
use tca6408a::registers::RegisterAddress as Reg;
use tca6408a_sim::{FakeI2c, Tca6408aModel};

/// Records every transaction on the bus: `Some(bytes)` for writes, `None` for reads.
struct Recorder<'a> {
    bus: FakeI2c<'a>,
    log: &'a RefCell<Vec<Option<Vec<u8>>>>,
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
            self.log.borrow_mut().push(match op {
                Operation::Write(bytes) => Some(bytes.to_vec()),
                Operation::Read(_) => None,
            });
        }

        self.bus.transaction(address, operations)
    }
}

#[test]
fn set_and_clear_several_pins() {
    let model = RefCell::new(Tca6408aModel::new());
    let io = Tca6408a::new(FakeI2c::new(&model))
        .into_shared::<RefCell<_>>()
        .unwrap();
    let mut pins = io.split();

    pins.p0.make_output(false).unwrap();
    pins.p3.make_output(true).unwrap();
    pins.p7.make_output(false).unwrap();
    assert_eq!(model.borrow().outputs(), 0x89);
    assert_eq!(model.borrow().pins() & 0x89, 0x08);

    pins.p0.set_high().unwrap();
    pins.p7.set_high().unwrap();
    pins.p3.set_low().unwrap();
    assert_eq!(model.borrow().pins() & 0x89, 0x81);
    assert!(pins.p0.is_set_high().unwrap());
    assert!(pins.p3.is_set_low().unwrap());

    pins.p7.toggle().unwrap();
    assert_eq!(model.borrow().pins() & 0x80, 0x00);

    pins.p3.make_input().unwrap();
    assert!(!pins.p3.is_output());
    assert_eq!(model.borrow().register(Reg::CONFIGURATION), 0x7E);
}

#[test]
fn input_reads_honor_polarity() {
    let model = RefCell::new(Tca6408aModel::new());
    let io = Tca6408a::new(FakeI2c::new(&model))
        .into_shared::<RefCell<_>>()
        .unwrap();
    io.with_driver(|d| d.write_polarity_inversion(0x01))
        .unwrap();
    let mut pins = io.split();

    // P0 is an active-low button, inverted to read high while pressed.
    assert!(pins.p0.is_low().unwrap());
    model.borrow_mut().drive(0xFE);
    assert!(model.borrow().interrupt());
    assert!(pins.p0.is_high().unwrap());
    assert!(!model.borrow().interrupt());
    assert!(pins.p1.is_high().unwrap());
}

#[test]
fn make_output_is_glitch_free() {
    let model = RefCell::new(Tca6408aModel::new());
    let log = RefCell::new(Vec::new());
    let io = Tca6408a::new(Recorder {
        bus: FakeI2c::new(&model),
        log: &log,
    })
    .into_shared::<RefCell<_>>()
    .unwrap();
    let mut pins = io.split();
    log.borrow_mut().clear();

    pins.p7.make_output(true).unwrap();
    assert_eq!(
        *log.borrow(),
        [Some(vec![0x01, 0xFF]), Some(vec![0x03, 0x7F])]
    );
}

#[test]
fn pin_writes_use_the_cache() {
    let model = RefCell::new(Tca6408aModel::new());
    let log = RefCell::new(Vec::new());
    let io = Tca6408a::new(Recorder {
        bus: FakeI2c::new(&model),
        log: &log,
    })
    .into_shared::<RefCell<_>>()
    .unwrap();
    let mut pins = io.split();
    pins.p6.make_output(true).unwrap();
    pins.p7.make_output(true).unwrap();
    log.borrow_mut().clear();

    // One write per change, no register reads; each pin keeps the other's bit.
    pins.p6.set_low().unwrap();
    pins.p7.set_low().unwrap();
    assert!(pins.p7.is_set_low().unwrap());
    assert_eq!(
        *log.borrow(),
        [Some(vec![0x01, 0xBF]), Some(vec![0x01, 0x3F])]
    );
}

#[test]
fn handles_share_state() {
    let model = RefCell::new(Tca6408aModel::new());
    let io = Tca6408a::new(FakeI2c::new(&model))
        .into_shared::<RefCell<_>>()
        .unwrap();
    let mut first = io.split();
    let mut second = io.split();

    first.p1.make_output(false).unwrap();
    second.p2.make_output(false).unwrap();
    assert!(second.p1.is_output());
    second.p1.set_high().unwrap();
    assert!(first.p1.is_set_high().unwrap());
    assert_eq!(model.borrow().register(Reg::OUTPUT_PORT), 0xFB);
    assert_eq!(model.borrow().register(Reg::CONFIGURATION), 0xF9);

    let mut driver = io.release();
    assert_eq!(driver.read_configuration().unwrap(), 0xF9);
}

#[test]
fn cache_starts_from_the_chip() {
    let model = RefCell::new(Tca6408aModel::new());
    let mut driver = Tca6408a::new(FakeI2c::new(&model));
    driver.make_outputs(0x80, 0x00).unwrap();
    let io = driver.into_shared::<RefCell<_>>().unwrap();
    let mut pins = io.split();
    assert!(pins.p7.is_output());
    assert!(pins.p7.is_set_low().unwrap());
    assert_eq!(pins.p7.mask(), 0x80);

    pins.p0.make_output(true).unwrap();
    assert_eq!(model.borrow().register(Reg::CONFIGURATION), 0x7E);
}

#[test]
fn errors_leave_the_cache_untouched() {
    let model = RefCell::new(Tca6408aModel::new());
    let io = Tca6408a::new(FakeI2c::new(&model))
        .into_shared::<RefCell<_>>()
        .unwrap();
    let mut pins = io.split();

    model.borrow_mut().set_reset_pin(false);
    assert!(pins.p4.make_output(false).is_err());
    assert!(pins.p4.set_low().is_err());
    assert!(pins.p4.is_high().is_err());
    model.borrow_mut().set_reset_pin(true);
    assert!(!pins.p4.is_output());
    assert!(pins.p4.is_set_high().unwrap());
}

#[cfg(feature = "critical-section")]
#[test]
fn critical_section_mutex() {
    let model = RefCell::new(Tca6408aModel::new());
    let io = Tca6408a::new(FakeI2c::new(&model))
        .into_shared::<critical_section::Mutex<RefCell<_>>>()
        .unwrap();
    let mut pins = io.split();
    pins.p5.make_output(false).unwrap();
    assert_eq!(model.borrow().outputs(), 0x20);
    assert_eq!(io.release().read_outputs().unwrap(), 0xDF);
}
