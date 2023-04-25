#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod w25q32jv;
pub mod w25q32jv_async;

/// Low level driver for the w25q32jv flash memory chip.
pub struct W25q32jv<SPI, CS, HOLD, WP> {
    spi: SPI,
    cs: CS,
    hold: HOLD,
    wp: WP,
}

/// Custom error type for the various errors that can be thrown by W25q32jv.
/// Can be converted into a NorFlashError.
#[derive(Debug)]
pub enum Error<S, P> {
    SpiError(S),
    PinError(P),
    NotAligned,
    OutOfBounds,
}

/// Easily readable representation of the command bytes used by the flash chip.
enum Command {
    Write = 0x02,
    Read = 0x03,
    ReadStatusRegister1 = 0x05,
    WriteEnable = 0x06,
    SectorErase = 0x20,
    DeviceID = 0x4B,
    Block32Erase = 0x52,
    Block64Erase = 0xD8,
    ChipErase = 0xC7,
}
