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

const WIFI_SSID: &str = drogue::config!("wifi-ssid");
const WIFI_PSK: &str = drogue::config!("wifi-password");
const HOST: &str = "192.168.1.2";
const PORT: u16 = 8088;
const USERNAME: &str = drogue::config!("http-username");
const PASSWORD: &str = drogue::config!("http-password");

bind_bsp!(Iot01a, BSP);

type Network = AdapterActor<EsWifi>;
type Sensor = Temperature<Hts221Ready, Hts221<Address<I2cPeripheral<I2c2>>>, Celsius>;

static WIFI: ActorContext<Network> = ActorContext::new();
static I2C: ActorContext<I2cPeripheral<I2c2>> = ActorContext::new();
static SENSOR: ActorContext<Sensor> = ActorContext::new();
static BUTTON: ActorContext<Button<UserButton, ButtonPressed<App>>> = ActorContext::new();
static APP: ActorContext<App> = ActorContext::new();

#[embassy::main(config = "Iot01a::config(true)")]
async fn main(s: Spawner, p: Peripherals) {
    // Configure board
    let board = Iot01a::new(p);

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

    let network = WIFI.mount(s, AdapterActor::new(wifi));
    let i2c = I2C.mount(s, I2cPeripheral::new(board.i2c2));
    let sensor = SENSOR.mount(s, Temperature::new(board.hts221_ready, Hts221::new(i2c)));
    let app = APP.mount(s, App::new(network, sensor));
    BUTTON.mount(
        s,
        Button::new(board.user_button, ButtonPressed(app, AppCommand::Send)),
    );

    defmt::info!("Application initialized. Press 'User' button to send data");
}

pub struct App {
    network: Address<Network>,
    sensor: Address<Sensor>,
}
impl App {
    pub fn new(network: Address<Network>, sensor: Address<Sensor>) -> Self {
        Self { network, sensor }
    }
}

#[derive(Clone)]
pub enum AppCommand {
    Send,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
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

                        let mut client = HttpClient::new(
                            &mut self.network,
                            &DNS,
                            HOST,
                            PORT,
                            USERNAME,
                            PASSWORD,
                        );

                        let tx: String<128> = serde_json_core::ser::to_string(&data).unwrap();
                        let mut rx_buf = [0; 256];
                        let response = client
                            .request(
                                Request::post()
                                    // Pass on schema
                                    .path("/v1/foo?data_schema=urn:drogue:iot:temperature")
                                    .payload(tx.as_bytes())
                                    .content_type(ContentType::ApplicationJson),
                                &mut rx_buf[..],
                            )
                            .await;
                        match response {
                            Ok(response) => {
                                defmt::info!("Response status: {:?}", response.status);
                                if let Some(payload) = response.payload {
                                    let s = core::str::from_utf8(payload).unwrap();
                                    defmt::trace!("Payload: {}", s);
                                } else {
                                    defmt::trace!("No response body");
                                }
                            }
                            Err(e) => {
                                defmt::warn!("Error doing HTTP request: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

static DNS: StaticDnsResolver<'static, 2> = StaticDnsResolver::new(&[
    DnsEntry::new("localhost", IpAddress::new_v4(127, 0, 0, 1)),
    DnsEntry::new(
        "http.sandbox.drogue.cloud",
        IpAddress::new_v4(95, 216, 224, 167),
    ),
]);
