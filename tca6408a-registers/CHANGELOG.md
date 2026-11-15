# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-15

[0.1.1]: https://github.com/sunsided/tca6408a/releases/tag/v0.1.1

### Changed

- Dropped `#![deny(warnings)]`, so a warning added by a newer compiler can no longer
  fail the docs.rs build.

## [0.1.0] - 2026-11-15

[0.1.0]: https://github.com/sunsided/tca6408a/releases/tag/v0.1.0

### Added

- Register map following the TCA6408A datasheet (SCPS192E): Input, Output, Polarity
  Inversion and Configuration registers with their power-up defaults.
