# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-15

[0.1.0]: https://github.com/sunsided/tca6408a/releases/tag/v0.1.0

### Added

- Blocking driver on `embedded-hal` 1.0 and async driver on `embedded-hal-async` 1.0
  (`async` feature): port reads and writes, polarity inversion, pin direction,
  glitch-free switching to outputs, and hardware reset.
- Per-pin handles implementing the `embedded-hal` 1.0 `InputPin`, `OutputPin` and
  `StatefulOutputPin` traits (`pins` feature), sharing the driver and a cached latch through
  a `PortMutex`: `RefCell`, and `critical_section::Mutex` with the `critical-section`
  feature.
