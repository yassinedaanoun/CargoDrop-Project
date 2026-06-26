use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod ble_rendezvous;
pub mod lan_rendezvous;

use crate::ui::interaction::InteractionHandler;

use crate::user_info::UserInfo;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub ip: [u8; 4],
    pub port: u16,
    pub username: String,
}

pub type PeerMap = Arc<RwLock<HashMap<String, Peer>>>;

// @TODO
// The RendezVousManager will be in charge of handling the multiple "P2P" discovery means, and will enable switching between implementations
// LAN is the preffered method, but DNS-SD is blocked over some networks
// => when LAN impossible, fall back to bluetooth detection.
#[allow(dead_code)] // @todo implementation of LAN discovery is still to be done
pub enum RendezvousImpl {
    Lan,
    Bluetooth,
}

pub struct RendezvousManager;

impl RendezvousManager {
    // The current rendezvous implementation in use.
    pub const RENDEZVOUS_IMPL: RendezvousImpl = RendezvousImpl::Bluetooth;

    // discover devices using relevant implementation (by order of preference)
    pub async fn discover_manage(peers: PeerMap, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
        match Self::RENDEZVOUS_IMPL {
            RendezvousImpl::Lan => lan_rendezvous::LanRendezvous::discover(peers, handler).await,
            RendezvousImpl::Bluetooth => ble_rendezvous::BleRendezvous::discover(peers, handler).await,
        }
    }

    // advertise presence to others using relevant implementation (by order of preference)
    pub async fn advertise_manage(user: &UserInfo, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
        match Self::RENDEZVOUS_IMPL {
            RendezvousImpl::Lan => lan_rendezvous::LanRendezvous::advertise(user, handler).await,
            RendezvousImpl::Bluetooth => ble_rendezvous::BleRendezvous::advertise(user, handler).await,
        }
    }
}

// traits defining a rendezvous engine (allowing for discovery and advertising)
pub trait RendezvousTrait {
    async fn discover(peers: PeerMap, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>>;
    async fn advertise(user: &UserInfo, handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>>;
}
