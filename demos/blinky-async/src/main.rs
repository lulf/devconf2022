#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _;
use panic_probe as _;

use embassy::executor::Spawner;
use embassy::traits::gpio::WaitForAnyEdge;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Input, Level, Output, Pull, Speed},
    Peripherals,
};
use embedded_hal::digital::v2::{InputPin, OutputPin};

#[embassy::main]
async fn main(_s: Spawner, p: Peripherals) {
    let mut led = Output::new(p.PA5, Level::Low, Speed::VeryHigh);
    let mut button = ExtiInput::new(Input::new(p.PC13, Pull::Up), p.EXTI13);

    loop {
        button.wait_for_any_edge().await;
        if button.is_high().unwrap() {
            led.set_high().unwrap();
        } else {
            led.set_low().unwrap();
        }
    }
}
