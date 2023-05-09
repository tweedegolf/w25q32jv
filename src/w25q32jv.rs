use super::*;
use core::fmt::Debug;
use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::OutputPin;
use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

impl<S, P> NorFlashError for Error<S, P>
where
    S: Debug,
    P: Debug,
{
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Error::SpiError(_) => NorFlashErrorKind::Other,
            Error::PinError(_) => NorFlashErrorKind::Other,
            Error::NotAligned => NorFlashErrorKind::NotAligned,
            Error::OutOfBounds => NorFlashErrorKind::OutOfBounds,
        }
    }
}

impl<SPI, S, P, CS, HOLD, WP> ErrorType for W25q32jv<SPI, CS, HOLD, WP>
where
    SPI: Transfer<u8, Error = S> + Write<u8, Error = S>,
    CS: OutputPin<Error = P>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    type Error = Error<S, P>;
}

impl<SPI, S, P, CS, HOLD, WP> ReadNorFlash for W25q32jv<SPI, CS, HOLD, WP>
where
    SPI: Transfer<u8, Error = S> + Write<u8, Error = S>,
    CS: OutputPin<Error = P>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        Self::capacity()
    }
}

impl<SPI, S, P, CS, HOLD, WP> NorFlash for W25q32jv<SPI, CS, HOLD, WP>
where
    SPI: Transfer<u8, Error = S> + Write<u8, Error = S>,
    CS: OutputPin<Error = P>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    const WRITE_SIZE: usize = 1;

    const ERASE_SIZE: usize = Self::SECTOR_SIZE as usize;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.erase_range(from, to)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write(offset, bytes)
    }
}

impl<SPI, S, P, CS, HOLD, WP> W25q32jv<SPI, CS, HOLD, WP>
where
    SPI: Transfer<u8, Error = S> + Write<u8, Error = S>,
    CS: OutputPin<Error = P>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    pub fn new(spi: SPI, cs: CS, hold: HOLD, wp: WP) -> Result<Self, Error<S, P>> {
        let mut flash = W25q32jv { spi, cs, hold, wp };

        flash.cs.set_high().map_err(Error::PinError)?;
        flash.hold.set_high().map_err(Error::PinError)?;
        flash.wp.set_high().map_err(Error::PinError)?;

        Ok(flash)
    }

    /// The flash chip is unable to perform new commands while it is still working on a previous one. Especially erases take a long time.
    /// This function returns true while the chip is unable to respond to commands (with the exception of the busy command).
    pub fn busy(&mut self) -> Result<bool, Error<S, P>> {
        let mut buf: [u8; 3] = [0; 3];
        buf[0] = Command::ReadStatusRegister1 as u8;

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.transfer(&mut buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        Ok((buf[1] & 0x01) != 0)
    }

    /// Request the 64 bit id that is unique to this chip.
    pub fn device_id(&mut self) -> Result<[u8; 8], Error<S, P>> {
        let mut buf: [u8; 13] = [0; 13];
        buf[0] = Command::UniqueId as u8;

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.transfer(&mut buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        Ok(TryFrom::try_from(&buf[5..]).unwrap())
    }

    /// Reset the chip
    pub fn reset(&mut self) -> Result<(), Error<S, P>> {
        self.cs.set_low().map_err(Error::PinError)?;
        self.spi
            .write(&[Command::EnableReset as u8])
            .map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;
        self.cs.set_low().map_err(Error::PinError)?;
        self.spi
            .write(&[Command::Reset as u8])
            .map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;
        Ok(())
    }

    /// Reads a chunk of bytes from the flash chip.
    /// The number of bytes read is equal to the length of the buf slice.
    /// The first byte is read from the provided address. This address is then incremented for each following byte.
    ///
    /// # Arguments
    /// * `address` - Address where the first byte of the buf will be read.
    /// * `buf` - Slice that is going to be filled with the read bytes.
    pub fn read(&mut self, address: u32, buf: &mut [u8]) -> Result<(), Error<S, P>> {
        if address + buf.len() as u32 >= Self::N_PAGES * Self::PAGE_SIZE {
            return Err(Error::OutOfBounds);
        }

        let address_bytes = address.to_be_bytes();
        let command_buf: [u8; 4] = [
            Command::ReadData as u8,
            address_bytes[0],
            address_bytes[1],
            address_bytes[2],
        ];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.spi.transfer(buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        Ok(())
    }

    /// Sets the enable_write flag on the flash chip to true.
    /// Writes and erases to the chip only have effect when this flag is true.
    /// Each write and erase clears the flag, requiring it to be set to true again for the next command.
    fn enable_write(&mut self) -> Result<(), Error<S, P>> {
        let command_buf: [u8; 1] = [Command::WriteEnable as u8];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        Ok(())
    }

    /// Writes a chunk of bytes to the flash chip.
    /// The first byte is written to the provided address. This address is then incremented for each following byte.
    ///
    /// # Arguments
    /// * `address` - Address where the first byte of the buf will be written.
    /// * `buf` - Slice of bytes that will be written.
    pub fn write(&mut self, mut address: u32, mut buf: &[u8]) -> Result<(), Error<S, P>> {
        if address + buf.len() as u32 > Self::N_PAGES * Self::PAGE_SIZE {
            return Err(Error::OutOfBounds);
        }

        // Write first chunk, taking into account that given addres might
        // point to a location that is not on a page boundary,
        let chunk_len = (Self::PAGE_SIZE - (address & 0x000000FF)) as usize;
        let chunk_len = chunk_len.min(buf.len());
        self.write_page(address, &buf[..chunk_len])?;

        // Write rest of the chunks
        let mut chunk_len = chunk_len;
        loop {
            buf = &buf[chunk_len..];
            address += chunk_len as u32;
            chunk_len = buf.len().min(Self::PAGE_SIZE as usize);
            if chunk_len == 0 {
                break;
            }
            self.write_page(address, &buf[..chunk_len])?;
        }

        Ok(())
    }

    fn write_page(&mut self, address: u32, buf: &[u8]) -> Result<(), Error<S, P>> {
        // We don't support wrapping writes. They're scary
        if (address & 0x000000FF) + buf.len() as u32 > Self::PAGE_SIZE {
            return Err(Error::OutOfBounds);
        }

        self.enable_write()?;

        let address_bytes = address.to_le_bytes();
        let command_buf: [u8; 4] = [
            Command::PageProgram as u8,
            address_bytes[2],
            address_bytes[1],
            address_bytes[0],
        ];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.spi.write(buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        while self.busy().unwrap() {}

        Ok(())
    }

    /// Erases a range of sectors. The range is expressed in bytes. These bytes need to be a multiple of SECTOR_SIZE.
    /// If the range starts at SECTOR_SIZE * 3 then the erase starts at the fourth sector.
    /// All sectors are erased in the range [start_sector..end_sector].
    /// The start address may not be a higher value than the end address.
    ///
    /// # Arguments
    /// * `start_address` - Address of the first byte of the start of the range of sectors that need to be erased.
    /// * `end_address` - Address of the first byte of the end of the range of sectors that need to be erased.
    pub fn erase_range(&mut self, start_address: u32, end_address: u32) -> Result<(), Error<S, P>> {
        self.enable_write()?;

        if start_address % (Self::SECTOR_SIZE) != 0 {
            return Err(Error::NotAligned);
        }

        if end_address % (Self::SECTOR_SIZE) != 0 {
            return Err(Error::NotAligned);
        }

        if start_address > end_address {
            return Err(Error::OutOfBounds);
        }

        let start_sector = start_address / Self::SECTOR_SIZE;
        let end_sector = end_address / Self::SECTOR_SIZE;

        for sector in start_sector..end_sector {
            self.erase_sector(sector).unwrap();
        }

        Ok(())
    }

    /// Erases a single sector of flash memory with the size of SECTOR_SIZE.
    ///
    /// # Arguments
    /// * `index` - the index of the sector that needs to be erased. The address of the first byte of the sector is the provided index * SECTOR_SIZE.
    pub fn erase_sector(&mut self, index: u32) -> Result<(), Error<S, P>> {
        self.enable_write()?;

        if index >= Self::N_SECTORS {
            return Err(Error::OutOfBounds);
        }

        let address: u32 = index * Self::SECTOR_SIZE;

        let address_bytes = address.to_be_bytes();
        let command_buf: [u8; 4] = [
            Command::SectorErase as u8,
            address_bytes[0],
            address_bytes[1],
            address_bytes[2],
        ];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        while self.busy().unwrap() {}

        Ok(())
    }

    /// Erases a single block of flash memory with the size of BLOCK_32K_SIZE.
    ///
    /// # Arguments
    /// * `index` - the index of the block that needs to be erased. The address of the first byte of the block is the provided index * BLOCK_32K_SIZE.
    pub fn erase_block_32k(&mut self, index: u32) -> Result<(), Error<S, P>> {
        self.enable_write()?;

        if index >= Self::N_BLOCKS_32K {
            return Err(Error::OutOfBounds);
        }

        let address: u32 = index * Self::BLOCK_32K_SIZE;

        let address_bytes = address.to_be_bytes();
        let command_buf: [u8; 4] = [
            Command::Block32Erase as u8,
            address_bytes[0],
            address_bytes[1],
            address_bytes[2],
        ];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        while self.busy().unwrap() {}

        Ok(())
    }

    /// Erases a single block of flash memory with the size of BLOCK_64K_SIZE.
    ///
    /// # Arguments
    /// * `index` - the index of the block that needs to be erased. The address of the first byte of the block is the provided index * BLOCK_64K_SIZE.
    pub fn erase_block_64k(&mut self, index: u32) -> Result<(), Error<S, P>> {
        self.enable_write()?;

        if index >= Self::N_BLOCKS_64K {
            return Err(Error::OutOfBounds);
        }

        let address: u32 = index * Self::BLOCK_64K_SIZE;

        let address_bytes = address.to_be_bytes();
        let command_buf: [u8; 4] = [
            Command::Block64Erase as u8,
            address_bytes[0],
            address_bytes[1],
            address_bytes[2],
        ];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        while self.busy().unwrap() {}

        Ok(())
    }

    /// Erases all sectors on the flash chip.
    /// This is a very expensive operation.
    pub fn erase_chip(&mut self) -> Result<(), Error<S, P>> {
        self.enable_write()?;

        let command_buf: [u8; 1] = [Command::ChipErase as u8];

        self.cs.set_low().map_err(Error::PinError)?;
        self.spi.write(&command_buf).map_err(Error::SpiError)?;
        self.cs.set_high().map_err(Error::PinError)?;

        while self.busy().unwrap() {}

        Ok(())
    }

    /// Get the capacity of the flash chip in bytes.
    pub fn capacity() -> usize {
        (Self::N_PAGES * Self::PAGE_SIZE) as usize
    }
}
