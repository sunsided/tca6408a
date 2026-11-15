# TI TCA6408A I/O Expander Registers

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> A typed map of the TCA6408A's I²C-accessible registers.

Provides functions and named fields to read and modify the expander's registers,
built on [`hardware-registers`](https://crates.io/crates/hardware-registers).

The map follows the TI TCA6408A datasheet,
[SCPS192E](https://www.ti.com/lit/ds/symlink/tca6408a.pdf) (April 2009, revised January 2023).

## Sources

All facts are from SCPS192E.

| Fact | Source |
|------|--------|
| Address `0x20` (`ADDR` low) or `0x21` (`ADDR` high) | SCPS192E, 8.6.1, Figure 8-9 and Table 8-3 |
| Command byte selects the register; sent only in write transfers | SCPS192E, 8.6.2 |
| Registers `0x00` Input, `0x01` Output, `0x02` Polarity Inversion, `0x03` Configuration | SCPS192E, 8.6.2, Table 8-4 |
| Power-up defaults: Output `0xFF`, Polarity Inversion `0x00`, Configuration `0xFF`, Input from the pins | SCPS192E, 8.6.2, Table 8-4; 8.6.3, Tables 8-5 to 8-8 |
| Bit `n` belongs to `Pn` | SCPS192E, 8.5.1, Table 8-2; 8.6.3 |
| Input reflects the pins in both directions; writes have no effect | SCPS192E, 8.6.3 |
| Output reads return the latch, not the pin | SCPS192E, 8.6.3 |
| Polarity Inversion inverts pins defined as inputs | SCPS192E, 8.6.3 |
| Configuration: `1` input (high impedance), `0` output | SCPS192E, 8.6.3 |

The datasheet does not say what happens with command bytes other than `0x00`–`0x03`, and
documents no auto-increment of the command byte. Reads repeat the selected register
(Figure 8-8); the `tca6408a-sim` model assumes the same for writes.

See also the [`tca6408a`](https://crates.io/crates/tca6408a) driver.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/tca6408a-registers
[crates-link]: https://crates.io/crates/tca6408a-registers
[docs-image]: https://docs.rs/tca6408a-registers/badge.svg
[docs-link]: https://docs.rs/tca6408a-registers/
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
