mod rendezvous;
mod cli;
mod use_cases;
mod network;
mod ui;
mod user_info;
//securite
mod security;
//securite

use use_cases::AppUseCases;
use cli::Cli;
use clap::Parser;
use std::error::Error;
use user_info::UserInfo;
//securite
use crate::security::{SecureSession, GestionnaireIdentite};
use tokio::sync::Mutex;
use std::sync::Arc;
//securite

use network::file_transfer::PeerInfo;
use network::tcp_client::TcpClient;
use network::tcp_server::TcpServer;
use std::collections::HashMap;
use tokio::sync::RwLock;
use ui::interaction::InteractionHandler;
use ui::cli_handler::CliHandler;
//securite
lazy_static::lazy_static! {
    pub static ref SECURE_SESSION: Arc<Mutex<Option<SecureSession>>> = Arc::new(Mutex::new(None));
}
//securite

#[derive(Clone)]
struct App {
    peers: rendezvous::PeerMap,
    handler: Arc<dyn InteractionHandler>,
    user_info: Arc<RwLock<UserInfo>>,
}

impl App {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let user_info = UserInfo::load().await?;
        Ok(Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            handler: Arc::new(CliHandler::new()),
            user_info: Arc::new(RwLock::new(user_info)),
        })
    }
}

/// Use cases dependency passed to the cli component to run it
impl AppUseCases for App {
    async fn advertise(&self) -> Result<(), Box<dyn Error>> {
        //securite
        let session = SecureSession::new("cargodrop-advertiser".to_string()).await?;
        let empreinte = GestionnaireIdentite::creer_empreinte(
            session.identite.obtenir_cle_verification_locale().as_slice()
        );
        let identifiant_court = GestionnaireIdentite::creer_identifiant_court(&empreinte);
        let user = UserInfo::load().await?;
        let display_name = format!("{}_{}",user.username, identifiant_court);
        println!("Appareil: {}", display_name);
        *SECURE_SESSION.lock().await = Some(session);
        //securite
        let user_guard = self.user_info.read().await;
        rendezvous::RendezvousManager::advertise_manage(&user_guard, self.handler.clone()).await
    }

    async fn discover(&self) -> Result<(), Box<dyn Error>> {
        //securite
        let session = SecureSession::new("cargodrop-discoverer".to_string()).await?;
        *SECURE_SESSION.lock().await = Some(session);
        //securite
        let peers_clone = self.peers.clone();
        let handler_clone = self.handler.clone();
        
        rendezvous::RendezvousManager::discover_manage(peers_clone, handler_clone).await
    }

    async fn send(&self, ip: String, port: Option<u16>, file_path: String) -> Result<(), Box<dyn Error>> {
        let UserInfo { port: config_port, username, .. } = self.user_info.read().await.clone();
        let actual_port = port.unwrap_or(config_port);
         //securite
        let mut session = SECURE_SESSION.lock().await;
        if session.is_none() {
            *session = Some(SecureSession::new("cargodrop-sender".to_string()).await?);
        }
        
        let session = session.as_mut().ok_or("Session non disponible")?;
        
        // Activation du chiffrement
        let (_, cle_chiffrement_vec) = session.initier_handshake()?;
        let mut cle_array = [0u8; 32];
        cle_array.copy_from_slice(&cle_chiffrement_vec);
        session.activer_chiffrement(&cle_array);
        
        println!(" Chiffrement activé avec: {}", hex::encode(&cle_array[..8]));
         //securite
        let peer = PeerInfo {
            ip,
            port: actual_port,
            device_name: "receiver".to_string(),
        };

        let client = TcpClient::new(peer, username, self.handler.clone());
        client.send_file(&file_path)
    }

    async fn receive(&self) -> Result<(), Box<dyn Error>> {
        let UserInfo { port: actual_port, username, .. } = self.user_info.read().await.clone();
        //securite
        // Vérifier et initialiser SANS garder le lock
        let needs_init = {
            let guard = SECURE_SESSION.lock().await;
            guard.is_none()
        }; 
        
        if needs_init {
            let session = SecureSession::new("cargodrop-receiver".to_string()).await?;
            let mut guard = SECURE_SESSION.lock().await;
            *guard = Some(session);
        } 
        
        // Initier le handshake
        let cle_chiffrement = {
            let guard = SECURE_SESSION.lock().await;
            let session = guard.as_ref().ok_or("Session non disponible")?;
            let (_, cle) = session.initier_handshake()?;
            cle
        }; 
        
        //  Activer le chiffrement
        {
            let mut guard = SECURE_SESSION.lock().await;
            let session = guard.as_mut().ok_or("Session non disponible")?;
            session.activer_chiffrement(&cle_chiffrement);
            println!("🔐 Chiffrement activé avec: {}", hex::encode(&cle_chiffrement[..8]));
        } 
        //securite
        let server = TcpServer::new(actual_port, username, self.handler.clone());
        server.start()
    }

    async fn advertise_and_receive(&self) -> Result<(), Box<dyn Error>> {
        // Trigger advertisement in the background using the dedicated use case
        let app_clone = self.clone();   // increment reference count to the app. Clone is required because we need a 
                                        // variable that will be able to outlive the function into the tokio thread
        
        tokio::spawn(async move {
            if let Err(e) = app_clone.advertise().await {
                eprintln!("Background advertisement error: {}", e);
            }
        });

        // Start the receive server to listen for incoming files
        self.receive().await
    }

    /// launches a discovery for 20 seconds, then stops and repeatedly asks the user to select a peer to send files.
    async fn interactive_send(&self, file_path: String) -> Result<(), Box<dyn Error>> {
        // Clear previous peer list for a fresh start at the beginning of the command
        {
            let mut peers_guard = self.peers.write().await;
            peers_guard.clear();
        }

        println!("Recherche d'appareils pendant 20 secondes...");
        // initial discovery for 20 seconds
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(20), self.discover()).await;

        loop {
            let peer_infos: Vec<PeerInfo> = {
                let peers_guard = self.peers.read().await;
                peers_guard.values().map(|p| PeerInfo {
                    ip: format!("{}.{}.{}.{}", p.ip[0], p.ip[1], p.ip[2], p.ip[3]),
                    port: p.port,
                    device_name: p.username.clone(),
                }).collect()
            };

            // once peer have been searched, called the UI handler to select a peer
            if let Some(selected_peer) = self.handler.select_peer(&peer_infos) {
                if let Err(e) = self.send(selected_peer.ip, Some(selected_peer.port), file_path.clone()).await {
                    eprintln!("Transfer failed: {}", e);
                } else {
                    println!("Transfer complete!");
                }
                // stay in loop for next selection
            } else {
                break;
            }
        }
        
        Ok(())
    }

    // User info use cases
    async fn get_ip(&self) -> Result<(), Box<dyn Error>> {
        let user = self.user_info.read().await;
        println!("Local IP: {}", user.local_ip);
        Ok(())
    }

    async fn get_name(&self) -> Result<(), Box<dyn Error>> {
        let user = self.user_info.read().await;
        println!("Username: {}", user.username);
        Ok(())
    }

    async fn set_name(&self, name: String) -> Result<(), Box<dyn Error>> {
        let mut user = self.user_info.write().await;
        user.set_username(name).await?;
        println!("Username changed to: {}", user.username);
        Ok(())
    }

    async fn set_name_default(&self) -> Result<(), Box<dyn Error>> {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "cargo-user".to_string());
        
        let mut user = self.user_info.write().await;
        user.set_username(hostname.clone()).await?;
        println!("Username reset to hostname: {}", user.username);
        Ok(())
    }

    async fn get_port(&self) -> Result<(), Box<dyn Error>> {
        let user = self.user_info.read().await;
        println!("Port: {}", user.port);
        Ok(())
    }

    async fn set_port(&self, port: u16) -> Result<(), Box<dyn Error>> {
        let mut user = self.user_info.write().await;
        user.set_port(port).await?;
        println!("Port changed to: {}", port);
        Ok(())
    }

    async fn set_port_default(&self) -> Result<(), Box<dyn Error>> {
        const DEFAULT_PORT: u16 = 8080;
        let mut user = self.user_info.write().await;
        user.set_port(DEFAULT_PORT).await?;
        println!("Port reset to default: {}", DEFAULT_PORT);
        Ok(())
    }

    async fn info(&self) -> Result<(), Box<dyn Error>> {
        let user = self.user_info.read().await;
        user.display();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let app = App::new().await?;

    cli.run(&app).await?;

    Ok(())
}
