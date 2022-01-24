#![no_std]
#![no_main]
#![macro_use]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod app;
use app::*;

use defmt_rtt as _;
use panic_probe as _;

use drogue_device::drivers::sensors::hts221::Hts221;
use drogue_device::{
    actors::button::{Button, ButtonPressed},
    actors::i2c::I2cPeripheral,
    actors::sensors::Temperature,
    domain::temperature::Celsius,
    drivers::button::Button as ButtonDriver,
    traits::sensors::temperature::TemperatureSensor,
    *,
};
use embassy::executor::Spawner;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Input, Pull},
    i2c, interrupt,
    peripherals::{DMA1_CH4, DMA1_CH5, I2C2, PC13, PD15},
    time::Hertz,
    Peripherals,
};

type I2c2 = i2c::I2c<'static, I2C2, DMA1_CH4, DMA1_CH5>;
type UserButton = ButtonDriver<ExtiInput<'static, PC13>>;
type Hts221Ready = ExtiInput<'static, PD15>;
type Sensor = Temperature<Hts221Ready, Hts221<Address<I2cPeripheral<I2c2>>>, Celsius>;

#[embassy::main]
async fn main(s: Spawner, p: Peripherals) {
    // I2C setup
    let i2c2 = i2c::I2c::new(
        p.I2C2,
        p.PB10,
        p.PB11,
        interrupt::take!(I2C2_EV),
        p.DMA1_CH4,
        p.DMA1_CH5,
        Hertz(100_000),
    );

    // Sensor pins
    let hts221_ready_pin = Input::new(p.PD15, Pull::Down);
    let hts221_ready = ExtiInput::new(hts221_ready_pin, p.EXTI15);

    // Button
    let user_button = ButtonDriver::new(ExtiInput::new(Input::new(p.PC13, Pull::Up), p.EXTI13));

    // Actors
    static I2C: ActorContext<I2cPeripheral<I2c2>> = ActorContext::new();
    let i2c = I2C.mount(s, I2cPeripheral::new(i2c2));

    static SENSOR: ActorContext<Sensor> = ActorContext::new();
    let sensor = SENSOR.mount(s, Temperature::new(hts221_ready, Hts221::new(i2c)));

    static APP: ActorContext<App> = ActorContext::new();
    let app = APP.mount(s, App::new(sensor));

    static BUTTON: ActorContext<Button<UserButton, ButtonPressed<App>>> = ActorContext::new();
    BUTTON.mount(
        s,
        Button::new(user_button, ButtonPressed(app, AppCommand::ReadSensor)),
    );

    defmt::info!("Application initialized. Press 'User' button to read sensor");
}
