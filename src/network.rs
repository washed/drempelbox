use async_std::task::sleep;
use chrono::Utc;
use itertools::Itertools;
use network_manager::{AccessPoint, AccessPointCredentials, Device, DeviceType, NetworkManager};
use std::time::Duration;
use tracing::{debug, error, info};

fn find_device(manager: &NetworkManager) -> Result<Device, Box<dyn std::error::Error>> {
    let devices = manager.get_devices()?;

    let index = devices
        .iter()
        .position(|d| *d.device_type() == DeviceType::WiFi);

    if let Some(index) = index {
        Ok(devices[index].clone())
    } else {
        Err(Box::<dyn std::error::Error>::from(
            "Cannot find a WiFi device",
        ))
    }
}

async fn get_ap_by_ssid(
    wd: &Device,
    ssid: &str,
    timeout: Duration,
) -> Result<AccessPoint, Box<dyn std::error::Error>> {
    let wd = wd.as_wifi_device().unwrap();

    let end = Utc::now() + timeout;

    info!("Seaching for AP {} for up to {:?}", ssid, timeout);
    wd.request_scan().unwrap();
    loop {
        let ap = wd
            .get_access_points()
            .unwrap()
            .into_iter()
            .filter(|ap| ap.ssid.as_str().unwrap() == ssid)
            .sorted_by_key(|ap| u32::MAX - ap.strength)
            .next();

        match (ap, Utc::now() >= end) {
            (Some(ap), true) => return Ok(ap),
            (ap, timed_out) => {
                debug!(ap=?ap, timed_out, "searching access point...");
                sleep(Duration::from_millis(500)).await
            }
        };
    }
}

fn cleanup_connections(nm: &NetworkManager, ssid: &str) -> Result<(), Box<dyn std::error::Error>> {
    let connections = nm.get_connections()?;
    connections
        .iter()
        .filter(|conn| conn.settings().ssid.as_str().unwrap() == ssid)
        .for_each(|conn| {
            debug!(
                "attempting to delete existing connection to AP with SSID {ssid}: {:?}",
                conn
            );
            match conn.delete() {
                Ok(_) => {}
                Err(e) => error!("error while trying to cleanup connections {}", e),
            }
        });

    Ok(())
}

fn connect_to_ap(
    wd: &Device,
    ap: &AccessPoint,
    credentials: &AccessPointCredentials,
) -> Result<(), Box<dyn std::error::Error>> {
    let wd = wd.as_wifi_device().unwrap();
    wd.connect(ap, credentials)?;
    Ok(())
}

pub async fn connect_wifi() -> Result<(), Box<dyn std::error::Error>> {
    const SSID: &str = "wpd";
    const PW: &str = "0987654321"; // TODO: get these via AP post req and/or NFC tag!

    let manager = NetworkManager::new();

    let device = find_device(&manager)?;

    let ap = get_ap_by_ssid(&device, SSID, Duration::from_secs(5)).await?;
    info!("found ap {:?}", ap);

    cleanup_connections(&manager, SSID);

    let credentials = AccessPointCredentials::Wpa {
        passphrase: PW.to_string(),
    };

    connect_to_ap(&device, &ap, &credentials)?;

    Ok(())
}
