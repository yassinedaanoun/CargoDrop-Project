use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
struct StoredConfig {
    pub username: String,
    pub port: u16,
}

/// Retourne le chemin du fichier de configuration
fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::home_dir()
        .ok_or("Impossible de trouver le répertoire home")?
        .join(".cargodrop");
    
    Ok(config_dir.join("config.json"))
}

/// Crée le répertoire de configuration s'il n'existe pas
async fn ensure_config_dir() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = dirs::home_dir()
        .ok_or("Impossible de trouver le répertoire home")?
        .join(".cargodrop");
    
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).await?;
    }
    Ok(())
}

/// Charge la configuration depuis le fichier JSON
pub async fn load_user_config() -> Result<(String, u16), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        return Err("Fichier de config non trouvé".into());
    }
    
    let content = fs::read_to_string(config_path).await?;
    let stored: StoredConfig = serde_json::from_str(&content)?;
    
    Ok((stored.username, stored.port))
}

/// Sauvegarde la configuration dans un fichier JSON
pub async fn save_user_config(username: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    ensure_config_dir().await?;
    
    let config_path = get_config_path()?;
    let stored = StoredConfig {
        username: username.to_string(),
        port,
    };
    
    let json = serde_json::to_string_pretty(&stored)?;
    fs::write(config_path, json).await?;
    
    Ok(())
}
