use std::error::Error;
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};

use crate::rendezvous::Peer;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use futures::stream::StreamExt;
use uuid::Uuid;
use std::sync::Arc;
use crate::ui::interaction::InteractionHandler;

use super::{APP_SERVICE_UUID, USERNAME_LEN_OFFSET, USERNAME_OFFSET};


pub struct BleDiscoveryService {
    peers: crate::rendezvous::PeerMap,
    handler: Arc<dyn InteractionHandler>,
}

impl BleDiscoveryService {
    pub fn new(peers: crate::rendezvous::PeerMap, handler: Arc<dyn InteractionHandler>) -> Self {
        Self { peers, handler }
    }

    /// Entry point for the discovery service.
    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        let adapter = self.setup_adapter().await?;
        
        // Run both the event stream and the monitoring task in parallel.
        // If this future is cancelled (e.g. by aborting the task), both sub-futures will be dropped and stop.
        let peers_monitor = self.peers.clone();
        let handler_monitor = self.handler.clone();
        
        tokio::select! {
            res = self.stream_events(adapter) => res,
            _ = Self::monitor_peers(peers_monitor, handler_monitor) => Ok(()),
        }
    }

    /// Sets up the Bluetooth manager and returns the first available hardware adapter.
    async fn setup_adapter(&self) -> Result<Adapter, Box<dyn Error>> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;

        if adapters.is_empty() {
            return Err("No Bluetooth adapters found on this system".into());
        }

        // Pick the first available Bluetooth adapter natively detected by the OS
        Ok(adapters.into_iter().nth(0).unwrap())
    }
}

/// Decodes the URL-safe Base64 payload into (IPv4, port, username).
/// Layout: [4 bytes IPv4][2 bytes port][1 byte username_len][N bytes username].
fn decode_network_info_from_name(name: &str) -> Option<([u8; 4], u16, String)> {
    let decoded = URL_SAFE_NO_PAD.decode(name).ok()?;

    if decoded.len() < USERNAME_OFFSET {
        return None;
    }

    let username_len = decoded[USERNAME_LEN_OFFSET] as usize;
    if decoded.len() != USERNAME_OFFSET + username_len {
        return None;
    }

    let mut ip = [0u8; 4];
    ip.copy_from_slice(&decoded[0..4]);

    let mut port_bytes = [0u8; 2];
    port_bytes.copy_from_slice(&decoded[4..6]);
    let port = u16::from_be_bytes(port_bytes);

    let username_bytes = &decoded[USERNAME_OFFSET..USERNAME_OFFSET + username_len];
    let username = String::from_utf8(username_bytes.to_vec()).ok()?;

    Some((ip, port, username))
}

impl BleDiscoveryService {
    /// display the detected peers list whenever a detection/disconnection happens
    async fn monitor_peers(peers: crate::rendezvous::PeerMap, handler: Arc<dyn InteractionHandler>) {
        let mut last_peers: std::collections::HashMap<String, crate::rendezvous::Peer> = std::collections::HashMap::new();
        let mut lost_peers: std::collections::HashMap<String, crate::rendezvous::Peer> = std::collections::HashMap::new();

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let p = peers.read().await;
            
            if p.len() != last_peers.len() {
                let time_str = chrono::Local::now().format("%H:%M:%S").to_string();
                
                // Identify new peers
                for (key, peer) in p.iter() {
                    if !last_peers.contains_key(key) {
                        handler.handle_peer_event(crate::ui::interaction::PeerEvent::NewPeer(peer.clone(), time_str.clone()));
                        // If it was previously lost, remove it from the lost list
                        lost_peers.remove(key);
                    }
                }
                
                // Identify lost peers
                for (key, peer) in last_peers.iter() {
                    if !p.contains_key(key) {
                        handler.handle_peer_event(crate::ui::interaction::PeerEvent::PeerLost(peer.clone(), time_str.clone()));
                        // Transfer to the local lost_peers list
                        lost_peers.insert(key.clone(), peer.clone());
                    }
                }

                // Display the current snapshot of all active peers and lost peers using the UI handler
                handler.display_peers_list(&*p, &lost_peers);
                
                last_peers = p.clone();
            }
        }
    }

    async fn stream_events(&self, adapter: Adapter) -> Result<(), Box<dyn Error>> {
        let target_uuid = Uuid::parse_str(APP_SERVICE_UUID)?;

        // Subscribe to the unbuffered native hardware event stream
        let mut events = adapter.events().await?;

        // Start continuous hardware scanning
        let scan_filter = ScanFilter {
            services: vec![target_uuid],
        };
        adapter.start_scan(scan_filter).await?;

        println!("Entering active CargoDrop BLE streaming loop...");

        // Track when the service started to filter out initial "ghost" OS cache dumps
        let app_start_time = tokio::time::Instant::now();

        // Key: encoded payload, Value: Last Seen Timestamp
        let mut heartbeats: std::collections::HashMap<String, tokio::time::Instant> =
            std::collections::HashMap::new();

        // Create an interval timer that ticks every 5 seconds to run our "Device Lost" disconnect logic
        let mut cleanup_interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(event) = events.next() => {
                    match &event {
                        // We react to events that indicate LIVE packets are arriving right now.
                        CentralEvent::DeviceDiscovered(id) |
                        CentralEvent::DeviceUpdated(id) |
                        CentralEvent::ManufacturerDataAdvertisement { id, .. } |
                        CentralEvent::ServiceDataAdvertisement { id, .. } |
                        CentralEvent::RssiUpdate { id, .. } => {
                            // We simulate "clearing the peers cache" from previous discoveries by ignoring packets in the 1st second
                            if let CentralEvent::DeviceDiscovered(_) = event {
                                if app_start_time.elapsed().as_millis() < 1000 {
                                    continue;
                                }
                            }

                            if let Ok(peripheral) = adapter.peripheral(id).await {
                                if let Some((payload_key, peer)) = self.filter_and_parse_peripheral(&peripheral, target_uuid).await {
                                    let now = tokio::time::Instant::now();
                                    self.handle_peer_found(payload_key.clone(), peer).await;
                                    
                                    // Insert or update the heartbeat timer
                                    heartbeats.insert(payload_key, now);
                                }
                            }
                        }
                        _ => {} // Ignore connection/disconnection events since we're connectionless
                    }
                }

                _ = cleanup_interval.tick() => {
                    let now = tokio::time::Instant::now();
                    
                    // Track keys to remove from shared map
                    let mut to_remove = Vec::new();
                    
                    heartbeats.retain(|name, last_seen| {
                        if now.duration_since(*last_seen).as_secs() > 20 {
                            to_remove.push(name.clone());
                            false // Drop from HashMap
                        } else {
                            true // Keep in HashMap
                        }
                    });
                    
                    // Update shared state
                    for name in to_remove {
                        self.handle_peer_lost(&name).await;
                    }
                }
            }
        }
    }

    async fn handle_peer_found(&self, key: String, peer: Peer) {
        let mut peers_write = self.peers.write().await;
        peers_write.insert(key, peer);
    }

    async fn handle_peer_lost(&self, key: &str) {
        let mut peers_write = self.peers.write().await;
        peers_write.remove(key);
    }

    /// Asynchronously fetches properties of a peripheral.
    /// Filters based on the App UUID, and then safely decodes its local name into a `Peer`.
    async fn filter_and_parse_peripheral(
        &self,
        peripheral: &Peripheral,
        target_uuid: Uuid,
    ) -> Option<(String, Peer)> {
        let properties = peripheral.properties().await.ok()??;

        // 1. Verify that this device is in our CargoDrop ecosystem
        //    (by checking the primary advertised service UUID)
        if !properties.services.contains(&target_uuid) {
            return None;
        }

        // 2. Extract the device's encoded local name chunk
        let local_name = properties.local_name?;

        // 3. Decode the base64 network info safely
        let (ip, port, username) = decode_network_info_from_name(&local_name)?;

        Some((
            local_name,
            Peer {
                ip,
                port,
                username,
            },
        ))
    }
}
