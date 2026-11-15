# TCA6408A on ESP32-C3

Reads buttons through a TCA6408A with the blocking `tca6408a` driver and prints them over
the serial console. The pin assignment follows the expander's use on the Kode Dot: P0–P4
are active-low buttons (BOT, up, left, down, right), P5 and P6 are interrupt inputs from
other chips, and P7 drives the `VCI_EN` panel supply.

At start-up, the firmware resets the expander and makes P7 an output with
`make_outputs(0x80, 0x80)`: it sets the output latch high before switching the
direction, so the supply is never driven low. The Configuration register then reads `0x7F`.
After that, it reads the inputs whenever `INT` goes low; the read clears the interrupt.

| ESP32-C3 | TCA6408A |
|----------|----------|
| GPIO4    | SDA      |
| GPIO5    | SCL      |
| GPIO6    | INT      |
| GPIO7    | RESET    |
| GND      | ADDR     |

On the Kode Dot, `INT` goes to GPIO6 of the ESP32-P4 and the bus runs at 400 kHz on I²C0
with SDA on GPIO4 and SCL on GPIO5. This example uses the same GPIO numbers on the ESP32-C3.
It also drives `RESET` from GPIO7; the board notes do not say how `RESET` is wired there.

## Run in Wokwi

Requires `rustup target add riscv32imc-unknown-none-elf wasm32-unknown-unknown`,
[`wokwi-cli`](https://docs.wokwi.com/wokwi-ci/cli-installation) and `WOKWI_CLI_TOKEN`.

```sh
../../sim/wokwi-chip/build.sh
cargo build --release
wokwi-cli . --scenario scenarios/buttons.yaml --timeout 30000
```

## Run on hardware

```sh
cargo run --release   # uses espflash
```

## Adding another board

Create `examples/<board>/` with its own firmware, `wokwi.toml` and `diagram.json`,
and reference the same chip binary (`../../sim/wokwi-chip/tca6408a.chip.wasm`).
