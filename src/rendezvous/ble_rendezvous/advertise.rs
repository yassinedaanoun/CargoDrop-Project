use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ble_peripheral_rust::{
    Peripheral, PeripheralImpl,
    gatt::{
        characteristic::Characteristic,
        properties::{AttributePermission, CharacteristicProperty},
        service::Service,
    },
};
use std::error::Error;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use uuid::Uuid;
use std::sync::Arc;
use crate::user_info::UserInfo;
use crate::ui::interaction::InteractionHandler;
use crate::security::GestionnaireIdentite;

use super::{APP_SERVICE_UUID, MAX_RAW_PAYLOAD_BYTES, USERNAME_OFFSET};

#[derive(Debug, Clone, Copy)]
struct AdvertiseConfig {
    adapter_power_poll: Duration,
    adapter_power_max_wait: Duration,
}

impl Default for AdvertiseConfig {
    fn default() -> Self {
        Self {
            adapter_power_poll: Duration::from_millis(50),
            adapter_power_max_wait: Duration::from_secs(60),
        }
    }
}

fn max_username_payload_bytes() -> usize {
    MAX_RAW_PAYLOAD_BYTES - USERNAME_OFFSET
}

fn truncate_username_for_payload(username: &str) -> String {
    let max_bytes = max_username_payload_bytes();
    if username.len() <= max_bytes {
        return username.to_string();
    }

    let mut end = max_bytes;
    while !username.is_char_boundary(end) {
        end -= 1;
    }

    username[..end].to_string()
}

/// Encodes IPv4, port and username into a compact raw payload and then Base64.
/// Layout: [4 bytes IPv4][2 bytes port][1 byte username_len][N bytes username].
fn encode_network_info_to_name(ipv4: [u8; 4], port: u16, username: &str) -> String {
    let truncated = truncate_username_for_payload(username);
    let username_bytes = truncated.as_bytes();

    let mut bytes = Vec::with_capacity(USERNAME_OFFSET + username_bytes.len());
    bytes.extend_from_slice(&ipv4);
    bytes.extend_from_slice(&port.to_be_bytes());
    bytes.push(username_bytes.len() as u8);
    bytes.extend_from_slice(username_bytes);

    URL_SAFE_NO_PAD.encode(bytes)
}

/// Initializes the BLE peripheral and the GATT service, returning the configured Peripheral.
async fn init_ble_peripheral(service_uuid: Uuid) -> Result<Peripheral, Box<dyn Error>> {
    let (sender_tx, mut receiver_rx) = mpsc::channel(256);
    let mut peripheral = Peripheral::new(sender_tx).await?;

    // Consume the channel events in a background task so it doesn't block.
    // For pure broadcasting, we don't care about interacting with clients via GATT requests.
    tokio::spawn(async move {
        while let Some(_event) = receiver_rx.recv().await {
            // Just dropping the events
        }
    });

    // Create a dummy service just to hold the primary app UUID.
    let service = Service {
        uuid: service_uuid,
        primary: true,
        characteristics: vec![Characteristic {
            uuid: Uuid::new_v4(),
            properties: vec![CharacteristicProperty::Read],
            permissions: vec![AttributePermission::Readable],
            ..Default::default()
        }],
    };

    peripheral.add_service(&service).await?;
    println!("GATT Service added locally.");

    Ok(peripheral)
}

async fn wait_until_adapter_powered(
    peripheral: &mut Peripheral,
    config: AdvertiseConfig,
) -> Result<(), Box<dyn Error>> {
    println!("Ensuring Bluetooth adapter is powered on...");

    let start = tokio::time::Instant::now();
    while !peripheral.is_powered().await? {
        if start.elapsed() >= config.adapter_power_max_wait {
            return Err("Timed out waiting for Bluetooth adapter to be powered on".into());
        }
        sleep(config.adapter_power_poll).await;
    }

    Ok(())
}

/// Discovers the local network config from UserInfo.
fn get_local_network_info(user: &UserInfo) -> ([u8; 4], u16) {
    (user.get_ip_bytes(), user.port)
}

fn get_local_username(user: &UserInfo) -> String {
    user.username.clone()
}

fn build_advertisement_payload(user: &UserInfo,identite: &crate::security::GestionnaireIdentite,) -> ([u8; 4], u16, String, String) {
    let (ip, port) = get_local_network_info(user);
    let username = get_local_username(user);
    let truncated_username = truncate_username_for_payload(&username);
    //security
    let cle_pub = identite.obtenir_cle_verification_locale();
    let empreinte = GestionnaireIdentite::creer_empreinte(&cle_pub);
    let identifiant_court = GestionnaireIdentite::creer_identifiant_court(&empreinte);
    let display_name = format!("{}_{}",truncated_username, identifiant_court);
    
    //security
    let device_name_payload = encode_network_info_to_name(ip, port, &display_name);
    println!(
        "Encoded rendezvous payload (IP: {}.{}.{}.{}, Port: {}, Username: '{}') -> Name: '{}'",
        ip[0], ip[1], ip[2], ip[3], port, display_name, device_name_payload
    );

    (ip, port, display_name, device_name_payload)
}

async fn start_advertising(
    peripheral: &mut Peripheral,
    service_uuid: Uuid,
    device_name_payload: &str,
) -> Result<(), Box<dyn Error>> {
    peripheral
        .start_advertising(device_name_payload, &[service_uuid])
        .await?;
    println!("Now actively advertising custom network rendezvous info...");
    Ok(())
}

/// The main advertising loop that continuously advertises the custom network rendezvous payload.
pub async fn advertise_rendezvous(user: &UserInfo, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
    let config = AdvertiseConfig::default();
    let service_uuid = Uuid::parse_str(APP_SERVICE_UUID)?;
    let identite = crate::security::GestionnaireIdentite::nouveau();

    // 1. Prepare payload components from UserInfo
    let (ip, port, username, device_name_payload) = build_advertisement_payload(user,&identite);

    // 2. Initialize BLE Peripheral & ++Service
    let mut peripheral = init_ble_peripheral(service_uuid).await?;
    wait_until_adapter_powered(&mut peripheral, config).await?;

    // 3. Start continuously advertising
    start_advertising(&mut peripheral, service_uuid, &device_name_payload).await?;

    // 4. Notify the UI handler that advertising has started
    handler.on_advertising_start(&username, ip, port, &device_name_payload);

    // 5. Keep process alive.
    // Since we want this to block (as it is currently used in use_cases), we loop forever.
    loop {
        sleep(Duration::from_secs(3600)).await;
    }
}
