#![no_std]
#![no_main]

use cortex_m_rt::entry;
use nrf9160_hal::{gpio, gpio::Level, pac::Peripherals, spim, Spim};
use nrf9160_rust_starter as _; // global logger + panicking-behavior + memory layout
use panic_probe as _;
use w25q32jv::W25q32jv;

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

    let mut buf: [u8; 16] = [0; 16];
    flash.read(0x00, &mut buf).unwrap();

    defmt::println!("Data: {:X}", buf[..]);

    nrf9160_rust_starter::exit()
}
