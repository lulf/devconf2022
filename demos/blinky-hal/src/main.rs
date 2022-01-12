#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_probe as _;

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embedded_hal::digital::v2::{InputPin, OutputPin};

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init(Default::default());
    let mut led = Output::new(p.PB14, Level::High, Speed::VeryHigh);
    let button = Input::new(p.PC13, Pull::Up);

    loop {
        if button.is_high().unwrap() {
            led.set_low().unwrap();
        } else {
            led.set_high().unwrap();
        }
    }
}
