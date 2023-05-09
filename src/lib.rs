#![no_std]
#![deny(unsafe_code)]
#![allow(incomplete_features)]
#![cfg_attr(feature = "async", feature(async_fn_in_trait))]

use embedded_hal::digital::OutputPin;
use embedded_storage::nor_flash::{ErrorType, NorFlashError, NorFlashErrorKind};
use core::fmt::Debug;

mod w25q32jv;
#[cfg(feature = "async")]
mod w25q32jv_async;

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

    /// Get the capacity of the flash chip in bytes.
    pub fn capacity() -> usize {
        (Self::N_PAGES * Self::PAGE_SIZE) as usize
    }
}

impl<SPI, S: Debug, P: Debug, HOLD, WP> W25q32jv<SPI, HOLD, WP>
where
    SPI: embedded_hal::spi::ErrorType<Error = S>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
{
    pub fn new(spi: SPI, hold: HOLD, wp: WP) -> Result<Self, Error<S, P>> {
        let mut flash = W25q32jv { spi, hold, wp };

        flash.hold.set_high().map_err(Error::PinError)?;
        flash.wp.set_high().map_err(Error::PinError)?;

        Ok(flash)
    }
}

impl<SPI, S: Debug, P: Debug, HOLD, WP> ErrorType for W25q32jv<SPI, HOLD, WP>
where
    SPI: embedded_hal::spi::ErrorType<Error = S>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
{
    type Error = Error<S, P>;
}

/// Custom error type for the various errors that can be thrown by W25q32jv.
/// Can be converted into a NorFlashError.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<S: Debug, P: Debug> {
    SpiError(S),
    PinError(P),
    NotAligned,
    OutOfBounds,
}

impl<S: Debug, P: Debug> NorFlashError for Error<S, P> {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Error::SpiError(_) => NorFlashErrorKind::Other,
            Error::PinError(_) => NorFlashErrorKind::Other,
            Error::NotAligned => NorFlashErrorKind::NotAligned,
            Error::OutOfBounds => NorFlashErrorKind::OutOfBounds,
        }
    }
}

/// Easily readable representation of the command bytes used by the flash chip.
enum Command {
    PageProgram = 0x02,
    ReadData = 0x03,
    ReadStatusRegister1 = 0x05,
    WriteEnable = 0x06,
    SectorErase = 0x20,
    UniqueId = 0x4B,
    Block32Erase = 0x52,
    Block64Erase = 0xD8,
    ChipErase = 0xC7,
    EnableReset = 0x66,
    Reset = 0x99,
}
