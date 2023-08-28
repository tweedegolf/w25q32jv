#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::{bind_interrupts, peripherals::SERIAL2, spim};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_storage_async::nor_flash::{NorFlash, ReadNorFlash};
use w25q32jv::W25q32jv;

bind_interrupts!(struct Irqs {
    UARTE2_SPIM2_SPIS2_TWIM2_TWIS2 => embassy_nrf::spim::InterruptHandler<SERIAL2>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M1;

    // Create the bus and the pins
    let spim = spim::Spim::new(p.SERIAL2, Irqs, p.P0_21, p.P0_24, p.P0_22, config);
    let cs = Output::new(p.P0_25, Level::Low, OutputDrive::Standard);
    let hold = Output::new(p.P0_20, Level::Low, OutputDrive::Standard);
    let wp = Output::new(p.P0_23, Level::Low, OutputDrive::Standard);

    let ed = ExclusiveDevice::new_no_delay(spim, cs);

    // Create the flash driver instance
    let mut flash = W25q32jv::new(ed, hold, wp).unwrap();

    // Embassy implements both eh-1 and eh-async, so we can use both blocking and async functions here
    flash.device_id_async().await.unwrap();
    flash.device_id().unwrap();

    // Erase the chip
    flash.erase_chip_async().await.unwrap();

    // The driver implements both sync and async NorFlash traits
    test_write(&mut flash).await;
    test_read(&mut flash).await;

    cortex_m::asm::udf();
}

const TEST_DATA: [u8; 4] = [0x36, 0x04, 0x81, 0xFE];
const TEST_OFFSET: u32 = 0x1000;

async fn test_write(flash: &mut impl NorFlash) {
    flash.write(TEST_OFFSET, &TEST_DATA).await.unwrap();
}

async fn test_read(flash: &mut impl ReadNorFlash) {
    let mut buf: [u8; 4] = [0; 4];
    flash.read(TEST_OFFSET, &mut buf).await.unwrap();

    assert_eq!(buf, TEST_DATA);
}

/// Called when our code panics.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::asm::udf();
}
