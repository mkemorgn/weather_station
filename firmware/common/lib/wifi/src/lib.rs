use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripheral,
    timer::EspTaskTimerService,
    wifi::{AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiEvent},
};
use log::{info, warn};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WifiError {
    #[error("Missing WiFi name")]
    MissingSsid,
    #[error("SSID too long")]
    SsidTooLong,
    #[error("Password too long")]
    PassTooLong,
    #[error("WiFi driver error: {0}")]
    DriverError(#[from] esp_idf_svc::sys::EspError),
}

pub async fn wifi(
    ssid: &str,
    pass: &str,
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<AsyncWifi<EspWifi<'static>>, WifiError> {
    if ssid.is_empty() {
        return Err(WifiError::MissingSsid);
    }

    let mut auth_method = AuthMethod::WPA2Personal;
    if pass.is_empty() {
        auth_method = AuthMethod::None;
        info!("Wifi password is empty");
    }

    let esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;
    let timer_service = EspTaskTimerService::new()?;
    let mut wifi = AsyncWifi::wrap(esp_wifi, sysloop.clone(), timer_service)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    info!("Starting wifi...");
    wifi.start().await?;

    info!("Scanning...");
    let ap_infos = wifi.scan().await?;
    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        warn!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().map_err(|_| WifiError::SsidTooLong)?,
        password: pass.try_into().map_err(|_| WifiError::PassTooLong)?,
        channel,
        auth_method,
        ..Default::default()
    }))?;

    info!("Connecting wifi...");
    wifi.connect().await?;

    info!("Waiting for DHCP lease...");
    wifi.wait_netif_up().await?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);

    // Subscribe to events to monitor connection health
    sysloop.subscribe::<WifiEvent, _>(move |event| {
        match event {
            WifiEvent::StaDisconnected(_) => {
                warn!("WiFi disconnected. ESP-IDF driver will attempt auto-reconnect.");
            }
            WifiEvent::StaConnected(_) => {
                info!("WiFi connected!");
            }
            _ => {}
        }
    })?;

    Ok(wifi)
}
