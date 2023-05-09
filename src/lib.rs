#![no_std]
#![deny(unsafe_code)]
#![feature(async_fn_in_trait)]

pub mod w25q32jv;
#[cfg(feature = "asynch")]
pub mod w25q32jv_async;

/// Low level driver for the w25q32jv flash memory chip.
pub struct W25q32jv<SPI, HOLD, WP> {
    spi: SPI,
    hold: HOLD,
    wp: WP,
}

impl<SPI, HOLD, WP> W25q32jv<SPI, HOLD, WP> {
    const PAGE_SIZE: u32 = 256;
    const N_PAGES: u32 = 16384;
    const SECTOR_SIZE: u32 = Self::PAGE_SIZE * 16;
    const N_SECTORS: u32 = Self::N_PAGES / 16;
    const BLOCK_32K_SIZE: u32 = Self::SECTOR_SIZE * 8;
    const N_BLOCKS_32K: u32 = Self::N_SECTORS / 8;
    const BLOCK_64K_SIZE: u32 = Self::BLOCK_32K_SIZE * 2;
    const N_BLOCKS_64K: u32 = Self::N_BLOCKS_32K / 2;
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

// pub trait Spi {

// }
