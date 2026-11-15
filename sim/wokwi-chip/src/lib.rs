//! Wokwi custom chip wrapping the `tca6408a-sim` behavioral model.
//!
//! Pins follow the TSSOP-16 pinout: `VCCI`, `ADDR`, `RESET`, `P0`–`P3`, `GND`, `P4`–`P7`,
//! `INT`, `SCL`, `SDA`, `VCCP`.
//!
//! - `P0`–`P7` drive the output latch where configured as outputs. Inputs have pull-ups,
//!   standing in for the external pull-ups the real part needs.
//! - `INT` is open drain and active low: it is pulled low while asserted and floats otherwise.
//! - `RESET` is active low, with a pull-up.
//! - `ADDR` selects address `0x20` (low or unconnected) or `0x21` (high). It is sampled once,
//!   at start-up.

use std::cell::RefCell;
use std::ffi::{CString, c_void};
use tca6408a_registers::{ALTERNATE_DEVICE_ADDRESS, DEFAULT_DEVICE_ADDRESS};
use tca6408a_sim::Tca6408aModel;
use wokwi_chip_ll::*;

struct Chip {
    model: Tca6408aModel,
    ports: [PinId; 8],
    int_pin: PinId,
    /// The pins currently driven as outputs, and their levels.
    outputs: u8,
    levels: u8,
    int_low: bool,
}

thread_local! {
    static CHIP: RefCell<Option<Chip>> = const { RefCell::new(None) };
}

fn with_chip<R>(f: impl FnOnce(&mut Chip) -> R) -> R {
    CHIP.with(|chip| f(chip.borrow_mut().as_mut().expect("chipInit not called")))
}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

fn debug(message: &str) {
    let message = cstr(message);
    // SAFETY: `message` is a valid NUL-terminated string for the duration of the call.
    unsafe { debugPrint(message.as_ptr()) };
}

impl Chip {
    /// Hands the levels at the pins to the model.
    fn sample(&mut self) {
        let levels = self.ports.iter().enumerate().fold(0u8, |acc, (bit, &pin)| {
            // SAFETY: Pin IDs were obtained from the simulator in `chipInit`.
            let high = unsafe { pinRead(pin) } == HIGH;
            acc | (u8::from(high) << bit)
        });
        self.model.drive(levels);
    }

    /// Updates the pins and `INT` from the model, then samples the inputs.
    fn sync(&mut self) {
        let outputs = self.model.outputs();
        let levels = self.model.pins() & outputs;

        for (bit, &pin) in self.ports.iter().enumerate() {
            let mask = 1 << bit;
            let changed = (outputs ^ self.outputs) & mask != 0
                || outputs & mask != 0 && (levels ^ self.levels) & mask != 0;

            if !changed {
                continue;
            }

            let mode = match (outputs & mask != 0, levels & mask != 0) {
                (true, true) => OUTPUT_HIGH,
                (true, false) => OUTPUT_LOW,
                (false, _) => INPUT_PULLUP,
            };
            // SAFETY: Pin IDs were obtained from the simulator in `chipInit`.
            unsafe { pinMode(pin, mode) };
        }

        self.outputs = outputs;
        self.levels = levels;
        self.sample();

        let int_low = self.model.interrupt();

        if int_low != self.int_low {
            self.int_low = int_low;
            // SAFETY: The pin ID was obtained from the simulator in `chipInit`.
            unsafe { pinMode(self.int_pin, if int_low { OUTPUT_LOW } else { INPUT }) };
        }
    }
}

extern "C" fn on_connect(_: *mut c_void, _address: u32, is_read: bool) -> bool {
    with_chip(|chip| chip.model.i2c_start(is_read))
}

extern "C" fn on_read(_: *mut c_void) -> u8 {
    with_chip(|chip| {
        chip.sample();
        let value = chip.model.i2c_read();
        chip.sync();
        value
    })
}

extern "C" fn on_write(_: *mut c_void, data: u8) -> bool {
    with_chip(|chip| {
        let ack = chip.model.i2c_write(data);
        chip.sync();
        ack
    })
}

extern "C" fn on_disconnect(_: *mut c_void) {}

extern "C" fn on_port_change(_: *mut c_void, _pin: PinId, _value: u32) {
    // Changes caused by `sync` itself arrive while the chip is borrowed; `sync` samples
    // the pins afterwards anyway.
    CHIP.with(|chip| {
        if let Ok(mut chip) = chip.try_borrow_mut() {
            chip.as_mut().map(Chip::sync);
        }
    });
}

extern "C" fn on_reset_change(_: *mut c_void, _pin: PinId, value: u32) {
    with_chip(|chip| {
        chip.model.set_reset_pin(value == HIGH);
        chip.sync();

        if value == LOW {
            debug("tca6408a: reset");
        }
    });
}

/// Entry point called by the simulator.
#[unsafe(no_mangle)]
pub extern "C" fn chipInit() {
    // SAFETY: All pointers passed to the simulator are valid for the duration of each
    // call; configs are copied by the simulator. Init functions are only called here.
    unsafe {
        let int_pin = pinInit(cstr("INT").as_ptr(), INPUT);
        let reset_pin = pinInit(cstr("RESET").as_ptr(), INPUT_PULLUP);
        let addr_pin = pinInit(cstr("ADDR").as_ptr(), INPUT);
        let sda = pinInit(cstr("SDA").as_ptr(), INPUT);
        let scl = pinInit(cstr("SCL").as_ptr(), INPUT);
        let ports =
            core::array::from_fn(|bit| pinInit(cstr(&format!("P{bit}")).as_ptr(), INPUT_PULLUP));

        let address = if pinRead(addr_pin) == HIGH {
            ALTERNATE_DEVICE_ADDRESS
        } else {
            DEFAULT_DEVICE_ADDRESS
        };
        let mut model = Tca6408aModel::with_address(address);
        model.set_reset_pin(pinRead(reset_pin) == HIGH);

        i2cInit(&I2CConfig {
            user_data: std::ptr::null(),
            address: u32::from(address),
            scl,
            sda,
            connect: on_connect,
            read: on_read,
            write: on_write,
            disconnect: on_disconnect,
        });

        pinWatch(
            reset_pin,
            &WatchConfig {
                user_data: std::ptr::null(),
                edge: BOTH,
                pin_change: on_reset_change,
            },
        );

        for &pin in &ports {
            pinWatch(
                pin,
                &WatchConfig {
                    user_data: std::ptr::null(),
                    edge: BOTH,
                    pin_change: on_port_change,
                },
            );
        }

        let mut chip = Chip {
            model,
            ports,
            int_pin,
            outputs: 0,
            levels: 0,
            int_low: false,
        };
        chip.sample();
        // Power-on reset: capture the initial input levels as the interrupt reference.
        chip.model.reset();
        chip.sync();

        CHIP.with(|c| *c.borrow_mut() = Some(chip));

        debug(&format!("tca6408a: chip initialized at 0x{address:02X}"));
    }
}
