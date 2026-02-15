use anyhow::Result;
use log::{info, error};

pub trait TelemetryProvider {
    fn read_telemetry(&mut self) -> Result<mqtt_messages::Telemetry>;
}

pub trait HeartbeatProvider {
    fn read_heartbeat(&self) -> Result<mqtt_messages::Heartbeat>;
}

pub trait MessageBus {
    fn publish(&mut self, topic: &str, payload: &[u8]) -> Result<()>;
}

pub struct Device<Telemetry, Heartbeat, Bus>
where
    Telemetry: TelemetryProvider,
    Heartbeat: HeartbeatProvider,
    Bus: MessageBus,
{
    provider: Telemetry,
    heartbeat: Heartbeat,
    bus: Bus,
    uuid: String,
}

impl<Telemetry, Heartbeat, Bus> Device<Telemetry, Heartbeat, Bus>
where
    Telemetry: TelemetryProvider,
    Heartbeat: HeartbeatProvider,
    Bus: MessageBus,
{
    pub fn new(provider: Telemetry, heartbeat: Heartbeat, bus: Bus, uuid: String) -> Self {
        Self {
            provider,
            heartbeat,
            bus,
            uuid,
        }
    }

    pub async fn run_loop(&mut self, interval_ms: u32, heartbeat_interval_ms: u32) -> Result<()> {
        info!("Device is running...");
        let telemetry_topic = mqtt_messages::sensor_data_topic(&self.uuid);
        let heartbeat_topic = mqtt_messages::heartbeat_topic(&self.uuid);

        let mut last_heartbeat = 0u32;

        loop {
            match self.provider.read_telemetry() {
                Ok(telemetry) => {
                    info!("Measured: {:.2}°C, {:.2}% RH", telemetry.temperature, telemetry.humidity);
                    let payload = serde_json::to_string(&telemetry)?;
                    let _ = self.bus.publish(&telemetry_topic, payload.as_bytes());
                }
                Err(e) => error!("Failed to read sensor: {:?}", e),
            }

            last_heartbeat += interval_ms;
            if last_heartbeat >= heartbeat_interval_ms {
                match self.heartbeat.read_heartbeat() {
                    Ok(hb) => {
                        info!("Heartbeat: RSSI: {}dBm, Uptime: {}s, Heap: {} bytes", hb.rssi, hb.uptime_secs, hb.free_heap);
                        let payload = serde_json::to_string(&hb)?;
                        let _ = self.bus.publish(&heartbeat_topic, payload.as_bytes());
                    }
                    Err(e) => error!("Failed to read heartbeat: {:?}", e),
                }
                last_heartbeat = 0;
            }

            esp_idf_svc::hal::delay::FreeRtos::delay_ms(interval_ms);
        }
    }
}
