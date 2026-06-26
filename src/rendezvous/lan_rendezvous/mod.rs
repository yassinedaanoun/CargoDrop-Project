use std::error::Error;
use std::sync::Arc;
use crate::rendezvous::{PeerMap, RendezvousTrait};
use crate::ui::interaction::InteractionHandler;
use crate::user_info::UserInfo;

pub struct LanRendezvous {}

impl RendezvousTrait for LanRendezvous {
    async fn advertise(_user: &UserInfo, _handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
        println!("LAN Rendezvous: Starting advertisement ...");
        Ok(())
    }
    
    async fn discover(_peers: PeerMap, _handler: Arc<dyn InteractionHandler>) -> Result<(), Box<dyn Error>> {
        println!("LAN Rendezvous: Starting discovery ...");
        Ok(())
    }
}
