#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_nrf::{spim, interrupt};
use embassy_time::{Duration, Timer};
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull};
use embedded_hal_async::spi::ExclusiveDevice;
use embedded_storage_async::nor_flash::{AsyncReadNorFlash, AsyncNorFlash};
use nrf9160_rust_starter as _; // global logger + panicking-behavior + memory layout
use w25q32jv::w25q32jv_async::W25q32jv;

#[embassy_executor::task]
async fn blink(pin: AnyPin) {
    let mut led = Output::new(pin, Level::Low, OutputDrive::Standard);

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(150)).await;
        led.set_low();
        Timer::after(Duration::from_millis(150)).await;
    }
}

async fn test_write(flash: &mut impl AsyncNorFlash) {
    let mut buf: [u8; 4] = [0x36, 0x04, 0x81, 0xFE];
    flash.write(0x00, &mut buf).await.unwrap();
}

async fn test_read(flash: &mut impl AsyncReadNorFlash) {
    let mut buf: [u8; 8] = [0; 8];
    flash.read(0x00, &mut buf).await.unwrap();

    defmt::println!("Data: {:X}", buf[..]);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let mut button = Input::new(p.P0_31, Pull::None);

    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M1;

    let irq = interrupt::take!(UARTE2_SPIM2_SPIS2_TWIM2_TWIS2);
    let spim = spim::Spim::new(p.UARTETWISPI2, irq, p.P0_21, p.P0_24, p.P0_22, config);

    let cs = Output::new(p.P0_25, Level::Low, OutputDrive::Standard);
    let hold = Output::new(p.P0_20, Level::Low, OutputDrive::Standard);
    let wp = Output::new(p.P0_23, Level::Low, OutputDrive::Standard);

    let ed = ExclusiveDevice::new(spim, cs);

    let mut flash = W25q32jv::new(ed, hold, wp).await.unwrap();

    spawner.spawn(blink(p.P0_02.degrade())).unwrap();

    cortex_m::asm::delay(100_000);

    test_write(&mut flash).await;

    loop {
        button.wait_for_low().await;

        test_read(&mut flash).await;

        button.wait_for_high().await;
    }
}
