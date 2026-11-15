# TI TCA6408A I/O Expander

[![Build Status](https://github.com/sunsided/tca6408a/workflows/Rust/badge.svg)](https://github.com/sunsided/tca6408a/actions)
[![codecov](https://codecov.io/gh/sunsided/tca6408a/graph/badge.svg)](https://codecov.io/gh/sunsided/tca6408a)

> Platform-agnostic Rust support for the TI TCA6408A 8-bit I²C I/O expander.

> **Status:** tested against a behavioral model of the chip (host tests and Wokwi), not yet
> verified on real hardware.

| Crate                                      | Description                                              |
|--------------------------------------------|----------------------------------------------------------|
| [`tca6408a-registers`](tca6408a-registers) | A typed map of the TCA6408A's I²C registers.             |
| [`tca6408a`](tca6408a)                     | An `embedded-hal` 1.0 driver (blocking and async).       |
| [`tca6408a-sim`](tca6408a-sim)             | A behavioral model of the chip for tests and emulation.  |

Everything follows the TI TCA6408A datasheet,
[SCPS192E](https://www.ti.com/lit/ds/symlink/tca6408a.pdf) (April 2009, revised January 2023).
No code was taken from other drivers.

## Sources

Register facts are listed in [`tca6408a-registers`](tca6408a-registers#sources). The
behavioral model adds:

| Fact | Source |
|------|--------|
| `INT` is open drain, active low | SCPS192E, 1 Features; 8.3.3 |
| `INT` asserts on any input edge; clears when the input returns or the Input register is read | SCPS192E, 8.1; 8.3.3 |
| Outputs never interrupt; switching an output to an input can | SCPS192E, 8.3.3 |
| Reading or writing other devices does not affect `INT` | SCPS192E, 8.3.3 |
| `RESET` low restores the register defaults and the I²C state machine | SCPS192E, 8.1; 8.3.4 |
| Power-on reset restores the defaults; all pins start as inputs | SCPS192E, 8.4.1; Figure 8-1 note B; Figure 8-2 note A |
| The command byte stays selected; further read bytes repeat the register | SCPS192E, 8.6.2; Figure 8-8 |
| Reset timing `tW` ≥ 4 ns, `tREC` = 0 ns, `tRESET` ≤ 600 ns | SCPS192E, 6.7 |
| Inputs are high impedance and need external pull-ups if they can float | SCPS192E, 8.3.2; Figure 9-1 note D |

## Assumptions

The datasheet leaves these points open. The model in `tca6408a-sim` behaves as follows:

- The command byte is `0x00` (Input register) after reset.
- Command bytes other than `0x00`–`0x03` are not acknowledged.
- Further data bytes of a write transfer go to the same register (no auto-increment).
- Polarity inversion applies only to pins configured as inputs; the Input register reports
  outputs at their driven level.
- Reset captures the current pin levels as the interrupt reference, so `INT` is released.
- While `RESET` is low, the chip does not acknowledge its address.
- The Wokwi chip samples `ADDR` once, at start-up, and pulls its inputs up (see
  [`sim/wokwi-chip`](sim/wokwi-chip)).

## Emulation

The behavioral model in `tca6408a-sim` is used both as a fake I²C bus in host tests and as a
[Wokwi](https://wokwi.com) custom chip (`sim/wokwi-chip`). Example firmware in `examples/`
runs against the emulated chip using `wokwi-cli`.

### Use the chip in your own Wokwi project

Every [GitHub Release](https://github.com/sunsided/tca6408a/releases) carries the chip as
`chip.zip`. Add a `chip-tca6408a` part and this dependency to your `diagram.json`, with the
latest release version written without the tag's `v` (`@v0.1.0` fails to load):

```json
"dependencies": { "chip-tca6408a": "github:sunsided/tca6408a@0.1.0" }
```

This works on [wokwi.com](https://wokwi.com), in the Wokwi VS Code extension and with
`wokwi-cli`; nothing needs to be built.

### Run the example locally

Needs [`wokwi-cli`](https://docs.wokwi.com/wokwi-ci/cli-installation) and a
`WOKWI_CLI_TOKEN` from the [Wokwi CI dashboard](https://wokwi.com/dashboard/ci):

```sh
task wokwi:test
```

[`sim/wokwi-chip`](sim/wokwi-chip/README.md) describes building the chip and its pins;
[`examples/esp32c3`](examples/esp32c3/README.md) the wiring and the scenario.

## Releasing

Both crates share one version. From `main`, with the changes listed under
`## [Unreleased]` in each crate's `CHANGELOG.md`:

```sh
cargo release <patch|minor|major>            # dry run
cargo release <patch|minor|major> --execute
```

This commits `Release X.Y.Z` and pushes the tag `vX.Y.Z`. The tag runs the release workflow,
which publishes `tca6408a-registers` and `tca6408a` to crates.io via
[trusted publishing](https://crates.io/docs/trusted-publishing) (no API token in the
repository) and then creates a GitHub Release named `X.Y.Z` with the changelog sections
as notes.

Each GitHub Release carries the packaged `.crate` files and the Wokwi chip as `chip.zip`,
all with [build provenance attestations](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations):

```sh
gh attestation verify chip.zip -R sunsided/tca6408a
```

## Code of Conduct

We abide by the [Contributor Covenant][cc] and ask that you do as well.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[cc]: https://contributor-covenant.org
