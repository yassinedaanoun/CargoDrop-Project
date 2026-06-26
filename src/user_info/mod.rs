mod network;
mod storage;

pub use network::get_local_ip;
pub use storage::{load_user_config, save_user_config};

use std::net::IpAddr;

/// Représente les informations de l'utilisateur local
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// Nom d'utilisateur (max 9 caractères pour BLE payload)
    pub username: String,
    /// Port HTTP pour le transfert
    pub port: u16,
    /// Adresse IP locale (détectée automatiquement)
    pub local_ip: IpAddr,
}

impl UserInfo {
    /// Crée une nouvelle configuration utilisateur avec les valeurs par défaut
    pub fn new(username: String, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let local_ip = get_local_ip()?;
        Ok(UserInfo {
            username: truncate_username(&username),
            port,
            local_ip,
        })
    }

    /// Charge la configuration utilisateur depuis le fichier de config
    /// ou crée une configuration par défaut
    pub async fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let local_ip = get_local_ip()?;
        
        match load_user_config().await {
            Ok((username, port)) => {
                Ok(UserInfo {
                    username,
                    port,
                    local_ip,
                })
            }
            Err(_) => {
                // Config par défaut si pas de fichier
                let hostname = hostname::get()
                    .ok()
                    .and_then(|h| h.into_string().ok())
                    .unwrap_or_else(|| "cargo-user".to_string());
                
                Self::new(hostname, 8080)
            }
        }
    }

    /// Sauvegarde la configuration utilisateur
    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        save_user_config(&self.username, self.port).await
    }

    /// Change le nom d'utilisateur et sauvegarde
    pub async fn set_username(&mut self, new_name: String) -> Result<(), Box<dyn std::error::Error>> {
        self.username = truncate_username(&new_name);
        self.save().await
    }

    /// Change le port et sauvegarde
    pub async fn set_port(&mut self, new_port: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.port = new_port;
        self.save().await
    }

    /// Retourne l'IP locale sous forme de tableau d'octets (pour BLE)
    pub fn get_ip_bytes(&self) -> [u8; 4] {
        match self.local_ip {
            IpAddr::V4(ipv4) => ipv4.octets(),
            IpAddr::V6(_) => [127, 0, 0, 1], // Fallback pour IPv6
        }
    }

    /// Affiche les infos utilisateur
    pub fn display(&self) {
        println!("=== User Configuration ===");
        println!("Username: {}", self.username);
        println!("Port: {}", self.port);
        println!("Local IP: {}", self.local_ip);
        println!("========================");
    }
}

/// Tronque le username à 14 caractères (limite du payload BLE héritée)
fn truncate_username(name: &str) -> String {
    name.chars().take(14).collect()
}
