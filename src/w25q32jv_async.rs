use super::{Command, Error};
use core::fmt::Debug;
use core::future::Future;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal_async::spi::{Operation, SpiDevice};
use embedded_storage;
use embedded_storage::nor_flash::ErrorType;
use embedded_storage_async::nor_flash::{AsyncNorFlash, AsyncReadNorFlash};

/// Async implementation of the low level driver for the w25q32jv flash memory chip.
pub struct W25q32jv<SPI, HOLD, WP> {
    spi: SPI,
    hold: HOLD,
    wp: WP,
}

impl<SPI, S, P, HOLD, WP> ErrorType for W25q32jv<SPI, HOLD, WP>
where
    SPI: SpiDevice<Error = S>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    type Error = Error<S, P>;
}

impl<SPI, S, P, HOLD, WP> AsyncReadNorFlash for W25q32jv<SPI, HOLD, WP>
where
    SPI: SpiDevice<Error = S>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    const READ_SIZE: usize = 1;

    type ReadFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
		Self: 'a;

    fn read<'a>(&'a mut self, offset: u32, bytes: &'a mut [u8]) -> Self::ReadFuture<'a> {
        async move { self.read(offset, bytes).await.and(Ok(())) }
    }

    fn capacity(&self) -> usize {
        Self::capacity()
    }
}

impl<SPI, S, P, HOLD, WP> AsyncNorFlash for W25q32jv<SPI, HOLD, WP>
where
    SPI: SpiDevice<Error = S>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    const WRITE_SIZE: usize = 1;

    const ERASE_SIZE: usize = Self::SECTOR_SIZE as usize;

    type EraseFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
	where
		Self: 'a;

    fn erase(&mut self, from: u32, to: u32) -> Self::EraseFuture<'_> {
        async move { self.erase_range(from, to).await.and(Ok(())) }
    }

    type WriteFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
	where
		Self: 'a;

    fn write<'a>(&'a mut self, offset: u32, bytes: &'a [u8]) -> Self::WriteFuture<'a> {
        async move { self.write(offset, bytes).await.and(Ok(())) }
    }
}

impl<SPI, S, P, HOLD, WP> W25q32jv<SPI, HOLD, WP>
where
    SPI: SpiDevice<Error = S>,
    HOLD: OutputPin<Error = P>,
    WP: OutputPin<Error = P>,
    S: Debug,
    P: Debug,
{
    const PAGE_SIZE: u32 = 256;
    const N_PAGES: u32 = 16384;
    const SECTOR_SIZE: u32 = Self::PAGE_SIZE * 16;
    const N_SECTORS: u32 = Self::N_PAGES / 16;
    const BLOCK_32K_SIZE: u32 = Self::SECTOR_SIZE * 8;
    const N_BLOCKS_32K: u32 = Self::N_SECTORS / 8;
    const BLOCK_64K_SIZE: u32 = Self::BLOCK_32K_SIZE * 2;
    const N_BLOCKS_64K: u32 = Self::N_BLOCKS_32K / 2;

    pub async fn new(spi: SPI, hold: HOLD, wp: WP) -> Result<Self, Error<S, P>> {
        let mut flash = W25q32jv { spi, hold, wp };

        flash.hold.set_high().map_err(Error::PinError)?;
        flash.wp.set_high().map_err(Error::PinError)?;

        Ok(flash)
    }

    /// The flash chip is unable to perform new commands while it is still working on a previous one. Especially erases take a long time.
    /// This function returns true while the chip is unable to respond to commands (with the exception of the busy command).
    pub async fn busy(&mut self) -> Result<bool, Error<S, P>> {
        let mut w_buf: [u8; 3] = [0; 3];
        w_buf[0] = Command::ReadStatusRegister1 as u8;

        let mut r_buf: [u8; 3] = [0; 3];

        self.spi
            .transfer(&mut r_buf, &w_buf)
            .await
            .map_err(Error::SpiError)?;

        Ok((r_buf[1] & 0x01) != 0)
    }

    /// Request the 64 bit id that is unique to this chip.
    pub async fn device_id(&mut self) -> Result<[u8; 8], Error<S, P>> {
        let mut w_buf: [u8; 13] = [0; 13];
        w_buf[0] = Command::UniqueId as u8;

        let mut r_buf: [u8; 13] = [0; 13];

        self.spi
            .transfer(&mut r_buf, &w_buf)
            .await
            .map_err(Error::SpiError)?;

        Ok(TryFrom::try_from(&r_buf[5..]).unwrap())
    }

    /// Reads a chunk of bytes from the flash chip.
    /// The number of bytes read is equal to the length of the buf slice.
    /// The first byte is read from the provided address. This address is then incremented for each following byte.
    ///
    /// # Arguments
    /// * `address` - Address where the first byte of the buf will be read.
    /// * `buf` - Slice that is going to be filled with the read bytes.   
    pub async fn read(&mut self, address: u32, buf: &mut [u8]) -> Result<(), Error<S, P>> {
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

        self.spi
            .transaction(&mut [Operation::Write(&command_buf), Operation::Read(buf)])
            .await
            .map_err(Error::SpiError)?;

        Ok(())
    }

    /// Sets the enable_write flag on the flash chip to true.
    /// Writes and erases to the chip only have effect when this flag is true.
    /// Each write and erase clears the flag, requiring it to be set to true again for the next command.
    async fn enable_write(&mut self) -> Result<(), Error<S, P>> {
        let command_buf: [u8; 1] = [Command::WriteEnable as u8];

        self.spi
            .write(&command_buf)
            .await
            .map_err(Error::SpiError)?;

        Ok(())
    }

    /// Writes a chunk of bytes to the flash chip.
    /// The first byte is written to the provided address. This address is then incremented for each following byte.
    ///
    /// # Arguments
    /// * `address` - Address where the first byte of the buf will be written.
    /// * `buf` - Slice of bytes that will be written.
    pub async fn write(&mut self, mut address: u32, mut buf: &[u8]) -> Result<(), Error<S, P>> {
        self.enable_write().await?;

        if address + buf.len() as u32 >= Self::N_PAGES * Self::PAGE_SIZE {
            return Err(Error::OutOfBounds);
        }

        // Write first chunk, taking into account that given addres might
        // point to a location that is not on a page boundary,
        let chunk_len = (Self::PAGE_SIZE - (address & 0x000000FF)) as usize;
        let chunk_len = chunk_len.min(buf.len());
        self.write_page(address, &buf[..chunk_len]).await?;

        // Write rest of the chunks
        let mut chunk_len = chunk_len;
        while !buf.is_empty() {
            buf = &buf[chunk_len..];
            address += chunk_len as u32;
            chunk_len = buf.len().min(Self::PAGE_SIZE as usize);
            self.write_page(address, &buf[..chunk_len]).await?;
        }

        Ok(())
    }

    /// Execute a write on a single page
    async fn write_page(&mut self, address: u32, buf: &[u8]) -> Result<(), Error<S, P>> {
        // We don't support wrapping writes. They're scary
        if (address & 0x000000FF) + buf.len() as u32 >= Self::PAGE_SIZE {
            return Err(Error::OutOfBounds);
        }

        let address_bytes = address.to_be_bytes();
        let command_buf: [u8; 4] = [
            Command::PageProgram as u8,
            address_bytes[0],
            address_bytes[1],
            address_bytes[2],
        ];

        self.spi
            .write(&command_buf)
            .await
            .map_err(Error::SpiError)?;
        self.spi.write(buf).await.map_err(Error::SpiError)?;

        self.spi
            .transaction(&mut [Operation::Write(&command_buf), Operation::Write(buf)])
            .await
            .map_err(Error::SpiError)?;

        while self.busy().await.unwrap() {}

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
    pub async fn erase_range(
        &mut self,
        start_address: u32,
        end_address: u32,
    ) -> Result<(), Error<S, P>> {
        self.enable_write().await?;

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
            self.erase_sector(sector).await.unwrap();
        }

        Ok(())
    }

    /// Erases a single sector of flash memory with the size of SECTOR_SIZE.
    ///
    /// # Arguments
    /// * `index` - the index of the sector that needs to be erased. The address of the first byte of the sector is the provided index * SECTOR_SIZE.
    pub async fn erase_sector(&mut self, index: u32) -> Result<(), Error<S, P>> {
        self.enable_write().await?;

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

        self.spi
            .write(&command_buf)
            .await
            .map_err(Error::SpiError)?;

        while self.busy().await.unwrap() {}

        Ok(())
    }

    /// Erases a single block of flash memory with the size of BLOCK_32K_SIZE.
    ///
    /// # Arguments
    /// * `index` - the index of the block that needs to be erased. The address of the first byte of the block is the provided index * BLOCK_32K_SIZE.
    pub async fn erase_block_32k(&mut self, index: u32) -> Result<(), Error<S, P>> {
        self.enable_write().await?;

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

        self.spi
            .write(&command_buf)
            .await
            .map_err(Error::SpiError)?;

        while self.busy().await.unwrap() {}

        Ok(())
    }

    /// Erases a single block of flash memory with the size of BLOCK_64K_SIZE.
    ///
    /// # Arguments
    /// * `index` - the index of the block that needs to be erased. The address of the first byte of the block is the provided index * BLOCK_64K_SIZE.
    pub async fn erase_block_64k(&mut self, index: u32) -> Result<(), Error<S, P>> {
        self.enable_write().await?;

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

        self.spi
            .write(&command_buf)
            .await
            .map_err(Error::SpiError)?;

        while self.busy().await.unwrap() {}

        Ok(())
    }

    /// Erases all sectors on the flash chip.
    /// This is a very expensive operation.
    pub async fn erase_chip(&mut self) -> Result<(), Error<S, P>> {
        self.enable_write().await?;

        let command_buf: [u8; 1] = [Command::ChipErase as u8];

        self.spi
            .write(&command_buf)
            .await
            .map_err(Error::SpiError)?;

        while self.busy().await.unwrap() {}

        Ok(())
    }

    /// Get the capacity of the flash chip in bytes.
    pub fn capacity() -> usize {
        (Self::N_PAGES * Self::PAGE_SIZE) as usize
    }
}
