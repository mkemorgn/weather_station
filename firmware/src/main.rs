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
    },
    mqtt::client::{EspMqttClient, MqttClientConfiguration},
    nvs::EspNvs,
};
use log::info;
use std::{thread::sleep, time::Duration};
use wifi::wifi;

//const UUID: &str = get_uuid::uuid();

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

    // Initialize NVS (necessary for Wi-Fi to work)
    let nvs_partition = EspNvsPartition::<NvsDefault>::take();
    let _nvs = EspNvs::<NvsDefault>::new(nvs_partition?, "wifi", true).unwrap();

    // Get peripherals from the ESP32
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // Load configuration from TOML file
    let app_config = CONFIG;

    // Connect to Wi-Fi network
    let _wifi = wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    )?;
    info!("Successfully connected to Wi-Fi");

    let uuid = get_uuid::uuid();

    info!("Our UUID is:");
    info!("{}", uuid);

    // Set up I2C communication for the sensor
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;
    let mut bme280 = BME280::new_primary(i2c);
    let mut delay = delay::Ets;

    match bme280.init(&mut delay) {
        Ok(_) => log::info!("Successfully initialized BME280 device"),
        Err(_) => log::error!("Failed to initialize BME280 device"),
    }

    // MQTT broker configuration
    let broker_url = if !app_config.mqtt_user.is_empty() {
        format!(
            "mqtt://{}:{}@{}",
            app_config.mqtt_user, app_config.mqtt_pass, app_config.mqtt_host
        )
    } else {
        format!("mqtt://{}", app_config.mqtt_host)
    };

    let mqtt_config = MqttClientConfiguration::default();

    // Create MQTT client
    let mut client = EspMqttClient::new_cb(&broker_url, &mqtt_config, move |_message_event| {})?;

    // Main loop for reading sensor data and publishing it
    loop {
        // Wait 1 second before reading data again
        sleep(Duration::from_secs(1));

        // Read temperature from the BME280 sensor
        let temp = bme280
            .measure(&mut delay)
            .map(|measurement| measurement.temperature)
            .unwrap_or_else(|_| 0.0);

        // Convert Temp to a string
        let temp_str = format!("{:.2}", temp);

        // Publish temperature data via MQTT
        client.enqueue(
            &mqtt_messages::temperature_data_topic(&uuid),
            QoS::AtLeastOnce,
            false,
            temp_str.as_bytes(),
        )?;

        // Optional: Log the temperature
        info!("Published temperature: {} °C", temp_str);
    }
}
