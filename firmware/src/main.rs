use anyhow::{Context, Result};
use bme280::i2c::BME280;
use embedded_svc::mqtt::client::QoS;
use esp_idf_svc::nvs::EspNvsPartition;
use esp_idf_svc::nvs::NvsDefault;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay,
        i2c::{I2cConfig, I2cDriver},
        peripherals::Peripherals,
        prelude::*,
        task::block_on,
    },
    mqtt::client::{EspMqttClient, MqttClientConfiguration, EventPayload},
    netif::IpEvent,
    nvs::EspNvs,
    wifi::{AsyncWifi, EspWifi},
};
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wifi::wifi;

mod device;
mod connection;
use device::{Device, HeartbeatProvider, MessageBus, TelemetryProvider};
use connection::ConnectionManager;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("localhost")]
    mqtt_host: &'static str,
    #[default("")]
    mqtt_user: &'static str,
    #[default("")]
    mqtt_pass: &'static str,
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

struct Bme280Handler<'d> {
    driver: BME280<I2cDriver<'d>>,
    delay: delay::Ets,
}

impl<'d> TelemetryProvider for Bme280Handler<'d> {
    fn read_telemetry(&mut self) -> Result<mqtt_messages::Telemetry> {
        let measurement = self
            .driver
            .measure(&mut self.delay)
            .map_err(|e| anyhow::anyhow!("BME280 error: {:?}", e))?;
        Ok(mqtt_messages::Telemetry {
            temperature: measurement.temperature,
            humidity: measurement.humidity,
        })
    }
}

struct HeartbeatHandler {
    wifi: std::sync::Arc<std::sync::Mutex<esp_idf_svc::wifi::AsyncWifi<esp_idf_svc::wifi::EspWifi<'static>>>>,
}

impl HeartbeatProvider for HeartbeatHandler {
    fn read_heartbeat(&self) -> Result<mqtt_messages::Heartbeat> {
        let rssi = if let Ok(wifi) = self.wifi.lock() {
            wifi.wifi().get_rssi().unwrap_or(0) as i32
        } else {
            0
        };

        let uptime_secs = unsafe { esp_idf_svc::sys::esp_timer_get_time() as u64 / 1_000_000 };
        let free_heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };

        Ok(mqtt_messages::Heartbeat {
            rssi,
            uptime_secs,
            free_heap,
        })
    }
}

struct MqttBus {
    client: EspMqttClient<'static>,
    broker_url: String,
    config: MqttClientConfiguration<'static>,
    connection_manager: ConnectionManager,
    wifi: Arc<Mutex<AsyncWifi<EspWifi<'static>>>>,
}

impl MqttBus {
    fn reconnect(&mut self) -> Result<()> {
        self.connection_manager.fail();
        let delay_val = self.connection_manager.current_backoff();

        warn!(
            "Network unstable, waiting {}s before next publish attempt...",
            delay_val
        );
        std::thread::sleep(Duration::from_secs(delay_val));

        // Proactively try to reconnect WiFi if it's down
        if let Ok(mut wifi) = self.wifi.lock() {
            if !wifi.is_connected().unwrap_or(false) {
                info!("WiFi is down. Attempting to reconnect WiFi first...");
                let _ = block_on(wifi.connect());
                
                // Wait up to 10 seconds for IP
                info!("Waiting for IP layer...");
                for _ in 0..10 {
                    if wifi.is_up().unwrap_or(false) {
                        info!("IP layer is up!");
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }

        info!("Attempting to reconnect MQTT...");
        let cm = self.connection_manager.clone();
        self.client = EspMqttClient::new_cb(&self.broker_url, &self.config, move |message_event| {
            match message_event.payload() {
                EventPayload::Connected(_) => {
                    info!("MQTT connected event received!");
                    cm.reset();
                }
                EventPayload::Disconnected => {
                    warn!("MQTT Disconnected event received.");
                    cm.mark_for_reconnect();
                }
                _ => {}
            }
        })
        .context("Failed to recreate MQTT client during reconnection")?;

        Ok(())
    }
}

impl MessageBus for MqttBus {
    fn publish(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        if self.connection_manager.should_reconnect() {
            self.reconnect()?;
        }

        let res = self
            .client
            .enqueue(topic, QoS::AtLeastOnce, false, payload);

        match res {
            Ok(_) => {
                // Only reset if we aren't flagged for a background reconnect
                if !self.connection_manager.should_reconnect() {
                    self.connection_manager.reset();
                }
                Ok(())
            }
            Err(e) => {
                error!("MQTT Publish Error: {}", e);
                self.reconnect()?;
                Err(e.into())
            }
        }
    }
}

fn main() -> Result<()> {
    // Initialize ESP-IDF system and logger
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Initialize NVS
    let nvs_partition = EspNvsPartition::<NvsDefault>::take()
        .context("Failed to take NVS partition")?;
    let _nvs = EspNvs::<NvsDefault>::new(nvs_partition, "wifi", true)
        .context("Failed to initialize NVS")?;

    let peripherals = Peripherals::take()
        .context("Failed to take peripherals")?;
    let sysloop = EspSystemEventLoop::take()
        .context("Failed to take system event loop")?;

    // Use block_on to run our async main logic
    block_on(async_main(peripherals, sysloop))
}

async fn async_main(peripherals: Peripherals, sysloop: EspSystemEventLoop) -> Result<()> {
    let app_config = CONFIG;

    // Connect to Wi-Fi
    let wifi_handle = wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop.clone(),
    )
    .await
    .context("Wi-Fi connection failed")?;
    info!("Successfully connected to Wi-Fi");

    let wifi = std::sync::Arc::new(std::sync::Mutex::new(wifi_handle));

    let uuid = get_uuid::uuid().to_string();
    info!("Our UUID is: {}", uuid);

    // Set up I2C and Sensor
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let i2c_config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &i2c_config)
        .context("Failed to initialize I2C driver")?;
    let mut bme280 = BME280::new_primary(i2c);
    let mut delay = delay::Ets;

    if let Err(e) = bme280.init(&mut delay) {
        error!("Failed to initialize BME280: {:?}", e);
    }
    let sensor = Bme280Handler {
        driver: bme280,
        delay,
    };

    let heartbeat = HeartbeatHandler { wifi: wifi.clone() };

    // MQTT Configuration
    let broker_url = format!("mqtt://{}", app_config.mqtt_host);
    let mut mqtt_config = MqttClientConfiguration::default();
    if !app_config.mqtt_user.is_empty() {
        mqtt_config.username = Some(app_config.mqtt_user);
    }
    if !app_config.mqtt_pass.is_empty() {
        mqtt_config.password = Some(app_config.mqtt_pass);
    }

    // Connection Manager handles the shared backoff state
    let connection_manager = ConnectionManager::new();
    let cm_clone = connection_manager.clone();
    let cm_wifi = connection_manager.clone();

    info!("Connecting to MQTT broker at: {}", broker_url);
    let client = EspMqttClient::new_cb(&broker_url, &mqtt_config, move |message_event| {
        match message_event.payload() {
            EventPayload::Connected(_) => {
                info!("MQTT connected event received!");
                cm_clone.reset();
            }
            EventPayload::Disconnected => {
                warn!("MQTT Disconnected event received.");
                cm_clone.mark_for_reconnect();
            }
            _ => {}
        }
    })
    .context("Failed to create MQTT client")?;

    let cm_ip = connection_manager.clone();

    // Monitor Wi-Fi state for proactive recovery
    sysloop.subscribe::<esp_idf_svc::wifi::WifiEvent, _>(move |event| {
        use esp_idf_svc::wifi::WifiEvent;
        match event {
            WifiEvent::StaDisconnected(_) => {
                warn!("WiFi disconnected event received.");
                cm_wifi.mark_for_reconnect();
            }
            WifiEvent::StaConnected(_) => {
                info!("WiFi connected event received!");
            }
            _ => {}
        }
    })?;

    // Monitor IP state
    sysloop.subscribe::<IpEvent, _>(move |event| {
        match event {
            IpEvent::DhcpIpAssigned(_) => {
                info!("IP address acquired!");
                cm_ip.reset();
            }
            _ => {}
        }
    })?;

    let bus = MqttBus {
        client,
        broker_url,
        config: mqtt_config,
        connection_manager,
        wifi,
    };

    let mut device = Device::new(sensor, heartbeat, bus, uuid);
    device.run_loop(1000, 60000).await
}
