#![no_std]
#![no_main]
#![macro_use]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(concat_idents)]

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

static I2C: ActorContext<I2cPeripheral<I2c2>> = ActorContext::new();
static SENSOR: ActorContext<Sensor> = ActorContext::new();
static BUTTON: ActorContext<Button<UserButton, ButtonPressed<App>>> = ActorContext::new();
static APP: ActorContext<App> = ActorContext::new();

#[embassy::main]
async fn main(s: Spawner, p: Peripherals) {
    let i2c2 = i2c::I2c::new(
        p.I2C2,
        p.PB10,
        p.PB11,
        interrupt::take!(I2C2_EV),
        p.DMA1_CH4,
        p.DMA1_CH5,
        Hertz(100_000),
    );

    let hts221_ready_pin = Input::new(p.PD15, Pull::Down);
    let hts221_ready = ExtiInput::new(hts221_ready_pin, p.EXTI15);

    let user_button = ButtonDriver::new(ExtiInput::new(Input::new(p.PC13, Pull::Up), p.EXTI13));

    let i2c = I2C.mount(s, I2cPeripheral::new(i2c2));
    let sensor = SENSOR.mount(s, Temperature::new(hts221_ready, Hts221::new(i2c)));
    let app = APP.mount(s, App::new(sensor));
    BUTTON.mount(
        s,
        Button::new(user_button, ButtonPressed(app, AppCommand::ReadSensor)),
    );

    defmt::info!("Application initialized. Press 'User' button to read sensor");
}

pub struct App {
    sensor: Address<Sensor>,
}
impl App {
    pub fn new(sensor: Address<Sensor>) -> Self {
        Self { sensor }
    }
}

#[derive(Clone)]
pub enum AppCommand {
    ReadSensor,
}

#[derive(Clone, Debug, defmt::Format)]
pub struct TemperatureData {
    pub temp: f32,
    pub hum: f32,
}

impl Actor for App {
    type Message<'m> = AppCommand;

    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl core::future::Future<Output = ()> + 'm;

    fn on_mount<'m, M>(
        &'m mut self,
        _: Address<Self>,
        inbox: &'m mut M,
    ) -> Self::OnMountFuture<'m, M>
    where
        M: Inbox<Self> + 'm,
        Self: 'm,
    {
        async move {
            loop {
                if let Some(_m) = inbox.next().await {
                    if let Ok(data) = self.sensor.temperature().await {
                        let data = TemperatureData {
                            temp: data.temperature.raw_value(),
                            hum: data.relative_humidity,
                        };
                        defmt::info!("Sensor data: {:?}", data);
                    }
                }
            }
        }
    }
}
