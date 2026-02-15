use anyhow::Result;
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
    mqtt::client::{EspMqttClient, MqttClientConfiguration},
    nvs::EspNvs,
};
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wifi::wifi;

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

fn main() -> Result<()> {
    // Initialize ESP-IDF system and logger
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Initialize NVS
    let nvs_partition = EspNvsPartition::<NvsDefault>::take();
    let _nvs = EspNvs::<NvsDefault>::new(nvs_partition?, "wifi", true).unwrap();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // Use block_on to run our async main logic
    block_on(async_main(peripherals, sysloop))
}

async fn async_main(peripherals: Peripherals, sysloop: EspSystemEventLoop) -> Result<()> {
    let app_config = CONFIG;

    // Connect to Wi-Fi
    let _wifi = wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop.clone(),
    )
    .await?;
    info!("Successfully connected to Wi-Fi");

    let uuid = get_uuid::uuid();
    info!("Our UUID is: {}", uuid);

    // Set up I2C
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;
    let mut bme280 = BME280::new_primary(i2c);
    let mut delay = delay::Ets;

    if let Err(e) = bme280.init(&mut delay) {
        error!("Failed to initialize BME280: {:?}", e);
    }

    // MQTT Configuration
    let broker_url = format!("mqtt://{}", app_config.mqtt_host);
    let mut mqtt_config = MqttClientConfiguration::default();
    if !app_config.mqtt_user.is_empty() {
        mqtt_config.username = Some(app_config.mqtt_user);
    }
    if !app_config.mqtt_pass.is_empty() {
        mqtt_config.password = Some(app_config.mqtt_pass);
    }

    info!("Connecting to MQTT broker at: {}", broker_url);
    let mut client = EspMqttClient::new_cb(&broker_url, &mqtt_config, move |_message_event| {})?;

    // Shared backoff state
    let backoff_secs = Arc::new(Mutex::new(1u64));
    let backoff_clone = backoff_secs.clone();

    // Monitor Wi-Fi state for backoff logic
    sysloop.subscribe::<esp_idf_svc::wifi::WifiEvent, _>(move |event| {
        use esp_idf_svc::wifi::WifiEvent;
        match event {
            WifiEvent::StaDisconnected(_) => {
                let mut delay = backoff_clone.lock().unwrap();
                warn!("WiFi disconnected. Next retry backoff: {}s", *delay);
                *delay = std::cmp::min(*delay * 2, 60);
            }
            WifiEvent::StaConnected(_) => {
                info!("WiFi connected! Resetting backoff.");
                let mut delay = backoff_clone.lock().unwrap();
                *delay = 1;
            }
            _ => {}
        }
    })?;

    loop {
        // Simple sleep for readability
        std::thread::sleep(Duration::from_secs(1));

        let measurement = bme280.measure(&mut delay);
        let temperature = measurement.as_ref().map(|m| m.temperature).unwrap_or(0.0);
        let humidity = measurement.as_ref().map(|m| m.humidity).unwrap_or(0.0);

        let data = mqtt_messages::Telemetry {
            temperature,
            humidity,
        };

        let payload = serde_json::to_string(&data)?;

        let res = client.enqueue(
            &mqtt_messages::sensor_data_topic(&uuid),
            QoS::AtLeastOnce,
            false,
            payload.as_bytes(),
        );

        if let Err(e) = res {
            error!("MQTT Publish Error: {}", e);
            
            let delay = *backoff_secs.lock().unwrap();
            if delay > 1 {
                warn!("Network might be unstable, waiting {}s before next publish attempt...", delay);
                std::thread::sleep(Duration::from_secs(delay));
            }

            info!("Attempting to reconnect MQTT...");
            client = EspMqttClient::new_cb(&broker_url, &mqtt_config, move |_message_event| {})?;
        } else {
            info!("Published: {:.2}°C, {:.2}% RH", temperature, humidity);
        }
    }
}
