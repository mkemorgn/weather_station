use anyhow::Result;
use mqtt_messages::Telemetry;
use log::{info, error};

pub trait TelemetryProvider {
    fn read_telemetry(&mut self) -> Result<Telemetry>;
}

pub trait MessageBus {
    fn publish(&mut self, topic: &str, payload: &[u8]) -> Result<()>;
}

pub struct App<P, B>
where
    P: TelemetryProvider,
    B: MessageBus,
{
    provider: P,
    bus: B,
    uuid: String,
}

impl<P, B> App<P, B>
where
    P: TelemetryProvider,
    B: MessageBus,
{
    pub fn new(provider: P, bus: B, uuid: String) -> Self {
        Self {
            provider,
            bus,
            uuid,
        }
    }

    pub async fn run_loop(&mut self, interval_ms: u32) -> Result<()> {
        info!("App is running...");
        let topic = mqtt_messages::sensor_data_topic(&self.uuid);

        loop {
            match self.provider.read_telemetry() {
                Ok(telemetry) => {
                    info!("Measured: {:.2}°C, {:.2}% RH", telemetry.temperature, telemetry.humidity);
                    
                    let payload = serde_json::to_string(&telemetry)?;
                    if let Err(e) = self.bus.publish(&topic, payload.as_bytes()) {
                        error!("Failed to publish telemetry: {:?}", e);
                    } else {
                        info!("Telemetry published successfully");
                    }
                }
                Err(e) => {
                    error!("Failed to read sensor: {:?}", e);
                }
            }

            esp_idf_svc::hal::delay::FreeRtos::delay_ms(interval_ms);
        }
    }
}
