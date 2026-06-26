use std::error::Error;
use std::sync::Arc;
use crate::rendezvous::{PeerMap, RendezvousTrait};
use crate::ui::interaction::InteractionHandler;
use crate::user_info::UserInfo;

pub mod advertise;
pub mod discover;

pub(crate) const APP_SERVICE_UUID: &str = "d59218d6-6b22-4a0b-9ba7-70e28148b488";

// Constants to define the structure of the BLE advertisement payload (stored in the "device name" field).
pub(crate) const NETWORK_INFO_BYTES: usize = 6;
pub(crate) const USERNAME_LEN_BYTES: usize = 1;
pub(crate) const USERNAME_LEN_OFFSET: usize = NETWORK_INFO_BYTES;
pub(crate) const USERNAME_OFFSET: usize = NETWORK_INFO_BYTES + USERNAME_LEN_BYTES;
pub(crate) const MAX_RAW_PAYLOAD_BYTES: usize = 32;

pub struct BleRendezvous {}

impl RendezvousTrait for BleRendezvous {
    async fn advertise(user: &UserInfo, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
        println!("BLE Rendezvous: Starting advertisement...");
        println!("Using user config: username='{}', port={}", user.username, user.port);
        advertise::advertise_rendezvous(user, handler).await
    }
    
    async fn discover(peers: PeerMap, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
        println!("BLE Rendezvous: Starting discovery...");
        let service = discover::BleDiscoveryService::new(peers, handler);
        service.run().await
    }
}
