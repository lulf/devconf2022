use super::*;

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
    // Actor command
    type Message<'m> = AppCommand;

    // Workaround until async traits
    type OnMountFuture<'m, M>
    where
        M: 'm,
    = impl core::future::Future<Output = ()> + 'm;

    // Actor entry point
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
