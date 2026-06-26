use crate::rendezvous::Peer;
use crate::network::file_transfer::PeerInfo;
use std::collections::HashMap;

/// Events related to the management of Peers
/// (using events like so allows to have one function handle_event instead of multiple ones)
pub enum PeerEvent {
    NewPeer(Peer, String),
    PeerLost(Peer, String),
}

/// trait in which to add the UI interactions : displaying information, requesting info from the user...
/// The CLI and GUI will both implement this, and will enable to call "handler.display ...." freely in the whole app
pub trait InteractionHandler: Send + Sync {
    fn display_peers_list(&self, active_peers: &HashMap<String, Peer>, lost_peers: &HashMap<String, Peer>);
    fn handle_peer_event(&self, event: PeerEvent);
    fn select_peer(&self, peers: &[PeerInfo]) -> Option<PeerInfo>;
    fn on_advertising_start(&self, username: &str, ip: [u8; 4], port: u16, device_name_payload: &str);
    fn update_progress(&self, message: &str, done: u64, total: u64);
}
