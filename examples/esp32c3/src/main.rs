//! Reads buttons through a TCA6408A and drives a supply enable, following the expander's use
//! on the Kode Dot.
//!
//! Wiring: SDA = GPIO4, SCL = GPIO5, INT = GPIO6, RESET = GPIO7; `ADDR` low (`0x20`).
//!
//! Expander pins: P0 BOT, P1 up, P2 left, P3 down and P4 right buttons (active low),
//! P5 IMU INT1, P6 touch IRQ, P7 output (`VCI_EN` panel supply).

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::main;
use esp_hal::time::Rate;
use esp_println::println;
use tca6408a::Tca6408a;

esp_bootloader_esp_idf::esp_app_desc!();

/// The `VCI_EN` panel supply enable.
const VCI_EN: u8 = 1 << 7;

/// The active-low buttons on P0 to P4.
const BUTTONS: [(&str, u8); 5] = [
    ("BOT", 1 << 0),
    ("UP", 1 << 1),
    ("LEFT", 1 << 2),
    ("DOWN", 1 << 3),
    ("RIGHT", 1 << 4),
];

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .expect("valid I2C config")
    .with_sda(peripherals.GPIO4)
    .with_scl(peripherals.GPIO5);
    let int = Input::new(
        peripherals.GPIO6,
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut rst = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());

    tca6408a::reset(&mut rst, &mut delay).unwrap();
    let mut io = Tca6408a::new(i2c);

    // The latch goes high before P7 becomes an output, so the supply never glitches off.
    io.make_outputs(VCI_EN, VCI_EN)
        .expect("expander responds after reset");
    let config = io.read_configuration().unwrap();
    let output = io.read_outputs().unwrap();
    println!("TCA6408A ok config=0x{config:02X} output=0x{output:02X}");

    loop {
        let inputs = io.read_inputs().unwrap();
        print_inputs(inputs);

        while int.is_high() {}
    }
}

fn print_inputs(inputs: u8) {
    println!(
        "inputs=0x{inputs:02X} vci_en={}",
        u8::from(inputs & VCI_EN != 0)
    );

    for (name, mask) in BUTTONS {
        if inputs & mask == 0 {
            println!("pressed {name}");
        }
    }
}
