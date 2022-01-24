use super::*;

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
    ReadSensor,
    ReadSensorAndReport,
}

#[derive(Clone, Serialize, Deserialize, Debug, defmt::Format)]
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
                if let Some(mut m) = inbox.next().await {
                    match *m.message() {
                        AppCommand::ReadSensor => {
                            if let Ok(data) = self.sensor.temperature().await {
                                let data = TemperatureData {
                                    temp: data.temperature.raw_value(),
                                    hum: data.relative_humidity,
                                };
                                defmt::info!("Sensor data: {:?}", data);
                            }
                        }
                        AppCommand::ReadSensorAndReport => {
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

                                let tx: String<128> =
                                    serde_json_core::ser::to_string(&data).unwrap();
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
    }
}

static DNS: StaticDnsResolver<'static, 2> = StaticDnsResolver::new(&[
    DnsEntry::new("localhost", IpAddress::new_v4(127, 0, 0, 1)),
    DnsEntry::new(
        "http.sandbox.drogue.cloud",
        IpAddress::new_v4(95, 216, 224, 167),
    ),
]);
