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
    actors::wifi::*,
    bsp::{boards::stm32l4::iot01a::*, Board},
    clients::http::*,
    domain::temperature::Celsius,
    drivers::dns::*,
    traits::ip::*,
    traits::sensors::temperature::TemperatureSensor,
    traits::wifi::*,
    *,
};
use embassy::executor::Spawner;
use embassy_stm32::Peripherals;
use heapless::String;
use serde::{Deserialize, Serialize};

mod app;
use app::*;

const WIFI_SSID: &str = drogue::config!("wifi-ssid");
const WIFI_PSK: &str = drogue::config!("wifi-password");
const HOST: &str = "192.168.1.2";
const PORT: u16 = 8088;
const USERNAME: &str = drogue::config!("http-username");
const PASSWORD: &str = drogue::config!("http-password");

bind_bsp!(Iot01a, BSP);

// Type aliases for convenience
type Network = AdapterActor<EsWifi>;
type Sensor = Temperature<Hts221Ready, Hts221<Address<I2cPeripheral<I2c2>>>, Celsius>;

#[embassy::main(config = "Iot01a::config(true)")]
async fn main(s: Spawner, p: Peripherals) {
    // Configure board from peripherals
    let board = Iot01a::new(p);

    // Wifi configuration
    let mut wifi = board.wifi;
    match wifi.start().await {
        Ok(()) => defmt::info!("Started..."),
        Err(err) => defmt::info!("Error... {}", err),
    }

    defmt::info!("Joining WiFi network...");
    wifi.join(Join::Wpa {
        ssid: WIFI_SSID.trim_end(),
        password: WIFI_PSK.trim_end(),
    })
    .await
    .expect("Error joining wifi");
    defmt::info!("WiFi network joined");

    // Actors
    static WIFI: ActorContext<Network> = ActorContext::new();
    let network = WIFI.mount(s, AdapterActor::new(wifi));

    static I2C: ActorContext<I2cPeripheral<I2c2>> = ActorContext::new();
    let i2c = I2C.mount(s, I2cPeripheral::new(board.i2c2));

    static SENSOR: ActorContext<Sensor> = ActorContext::new();
    let sensor = SENSOR.mount(s, Temperature::new(board.hts221_ready, Hts221::new(i2c)));

    static APP: ActorContext<App> = ActorContext::new();
    let app = APP.mount(s, App::new(network, sensor));

    static BUTTON: ActorContext<Button<UserButton, ButtonPressed<App>>> = ActorContext::new();
    BUTTON.mount(
        s,
        Button::new(
            board.user_button,
            ButtonPressed(app, AppCommand::ReadSensorAndReport),
        ),
    );

    defmt::info!("Application initialized. Press 'User' button to send data");
}
