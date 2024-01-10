# W25Q32JV Flash driver

[![crates.io](https://img.shields.io/crates/v/w25q32jv.svg)](https://crates.io/crates/w25q32jv) [![Documentation](https://docs.rs/w25q32jv/badge.svg)](https://docs.rs/w25q32jv)

This is a generic driver for the W25Q32JV flash chip from Winbond.

It supports:
- Blocking SPI using `embedded-hal 1.0`
- Async SPI using `embedded-hal-async`
- Blocking `embedded-storage`
- Async `embedded-storage-async`

To unlock the use of async, activate the `async` feature on the crate.

Defmt is also supported through the `defmt` feature.

## TODO

- Fast read support. So far there's only support for the normal read, so don't use a SPI speed of > 50Mhz

## Changelog

### Unreleased

### [0.4.0] - 2024-01-10

- Update to embedded-hal 1.0

### [0.3.2] - 2023-11-13 

- Added functions to use the power down mode of the W25Q32JV.

### [0.3.1] - 2023-10-24

- Added readback-check feature that reads back the writes and the erases to check if they've succeeded ok

### [0.3.0] - 2023-10-23

- *BREAKING*: Error struct is now exhaustive and a variant was added
- Write enable is now being checked

### [0.1.0] - 2023-05-11
- Initial release
