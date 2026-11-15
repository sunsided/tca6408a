# TI TCA6408A I/O Expander Driver

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> A platform-agnostic driver to interface with the TI TCA6408A 8-bit I²C I/O expander.

Built on [`embedded-hal`] 1.0 and the [`tca6408a-registers`] map. Enable the `async`
feature for a driver on [`embedded-hal-async`].

```rust,ignore
use tca6408a::Tca6408a;

let mut io = Tca6408a::new(i2c); // ADDR low: 0x20

// P7 drives high; the latch is written before the direction, so P7 never glitches low.
io.make_outputs(0b1000_0000, 0b1000_0000)?;

// Wait for INT to go low, then read (and clear) the inputs:
let inputs = io.read_inputs()?;
let p0_low = inputs & 0b0000_0001 == 0;
```

## Implemented so far

- Port-wide reads and writes of the inputs, output latch, polarity inversion and direction
- Masked helpers: `set_high`, `set_low`, `make_inputs`, and `make_outputs` that sets the
  output latch before the direction
- Hardware reset via the `RESET` pin with datasheet timing
- Both addresses (`ADDR` low or high)
- Direct access to registers via [`tca6408a-registers`] structs and [`hardware-registers`] traits

[`embedded-hal`]: https://crates.io/crates/embedded-hal
[`embedded-hal-async`]: https://crates.io/crates/embedded-hal-async
[`tca6408a-registers`]: https://crates.io/crates/tca6408a-registers
[`hardware-registers`]: https://crates.io/crates/hardware-registers

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/tca6408a
[crates-link]: https://crates.io/crates/tca6408a
[docs-image]: https://docs.rs/tca6408a/badge.svg
[docs-link]: https://docs.rs/tca6408a/
[build-image]: https://github.com/sunsided/tca6408a/workflows/Rust/badge.svg
[build-link]: https://github.com/sunsided/tca6408a/actions
[safety-image]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[safety-link]: https://github.com/rust-secure-code/safety-dance/
[msrv-image]: https://img.shields.io/badge/rustc-1.85+-blue.svg
[license-eupl-image]: https://img.shields.io/badge/license-EUPL_1.2-blue.svg
[license-eupl-link]: https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12
[license-apache-image]: https://img.shields.io/badge/license-Apache_2.0-blue.svg
[license-apache-link]: https://www.apache.org/licenses/LICENSE-2.0
[license-mit-image]: https://img.shields.io/badge/license-MIT-blue.svg
[license-mit-link]: https://opensource.org/licenses/MIT
