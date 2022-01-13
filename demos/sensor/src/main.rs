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
    bsp::{boards::stm32l4::iot01a::*, Board},
    domain::temperature::Celsius,
    traits::sensors::temperature::TemperatureSensor,
    *,
};
use embassy::executor::Spawner;
use embassy_stm32::Peripherals;

bind_bsp!(Iot01a, BSP);

type Sensor = Temperature<Hts221Ready, Hts221<Address<I2cPeripheral<I2c2>>>, Celsius>;

static I2C: ActorContext<I2cPeripheral<I2c2>> = ActorContext::new();
static SENSOR: ActorContext<Sensor> = ActorContext::new();
static BUTTON: ActorContext<Button<UserButton, ButtonPressed<App>>> = ActorContext::new();
static APP: ActorContext<App> = ActorContext::new();

#[embassy::main(config = "Iot01a::config(true)")]
async fn main(s: Spawner, p: Peripherals) {
    // Configure board
    let board = Iot01a::new(p);

    let i2c = I2C.mount(s, I2cPeripheral::new(board.i2c2));
    let sensor = SENSOR.mount(s, Temperature::new(board.hts221_ready, Hts221::new(i2c)));
    let app = APP.mount(s, App::new(sensor));
    BUTTON.mount(
        s,
        Button::new(
            board.user_button,
            ButtonPressed(app, AppCommand::ReadSensor),
        ),
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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
