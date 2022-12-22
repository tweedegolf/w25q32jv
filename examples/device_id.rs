#![no_std]
#![no_main]

use panic_probe as _;
use cortex_m_rt::entry;
use w25q32jv::W25q32jv;
use nrf9160_rust_starter as _; // global logger + panicking-behavior + memory layout
use nrf9160_hal::{gpio, spim, Spim, pac::Peripherals, gpio::Level};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let port0 = gpio::p0::Parts::new(peripherals.P0_NS);

    let cs = port0.p0_25.into_push_pull_output(Level::Low);
    let hold = port0.p0_20.into_push_pull_output(Level::Low);
    let wp = port0.p0_23.into_push_pull_output(Level::Low);

    let spiclk = port0.p0_21.into_push_pull_output(Level::Low).degrade();
    let spimosi = port0.p0_22.into_push_pull_output(Level::Low).degrade();
    let spimiso = port0.p0_24.into_floating_input().degrade();

    let pins = spim::Pins {
        sck: Some(spiclk),
        miso: Some(spimiso),
        mosi: Some(spimosi),
    };

    let spi = Spim::new(
        peripherals.SPIM2_NS,
        pins,
        spim::Frequency::M4,
        spim::MODE_0,
        0,
    );

    let mut flash = W25q32jv::new(spi, cs, hold, wp).unwrap();

    cortex_m::asm::delay(100_000);

    defmt::println!("Device ID: {:X}", flash.device_id().unwrap());

    loop {

    }

    //nrf9160_rust_starter::exit()
}
