#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _;
use panic_probe as _;

use drogue_device::{
    actors,
    drivers::{button::Button as ButtonDriver, led::Led as LedDriver, ActiveLow},
    ActorContext, Address,
};
use embassy::executor::Spawner;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Input, Level, Output, Pull, Speed},
    peripherals::{PB14, PC13},
    Peripherals,
};

type Led = actors::led::Led<LedDriver<Output<'static, PB14>>>;
type Button =
    actors::button::Button<ButtonDriver<ExtiInput<'static, PC13>, ActiveLow>, Address<Led>>;

#[embassy::main]
async fn main(s: Spawner, p: Peripherals) {
    static LED: ActorContext<Led> = ActorContext::new();
    static BUTTON: ActorContext<Button> = ActorContext::new();

    let led_address = LED.mount(
        s,
        Led::new(Output::new(p.PB14, Level::High, Speed::Low).into()),
    );
    BUTTON.mount(
        s,
        Button::new(
            ButtonDriver::new(ExtiInput::new(Input::new(p.PC13, Pull::Up), p.EXTI13)),
            led_address,
        ),
    );
}
