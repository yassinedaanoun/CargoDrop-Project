use std::net::{IpAddr, UdpSocket};

/// Détecte l'adresse IP locale en créant une socket UDP
/// 
/// La socket est "préparée" en se connectant à 8.8.8.8:80 (sans établir une vraie connexion).
/// Cela permet de récupérer l'adresse IP de l'interface réseau appropriée.
/// 
/// Important:
/// - Fonctionne SANS connexion Internet (pas de vrai lien vers Google DNS)
/// - Nécessite une connexion WiFi/LAN (échoue sans interface réseau disponible)
/// 
/// Retourne une erreur si aucune interface WiFi/LAN n'est disponible.
pub fn get_local_ip() -> Result<IpAddr, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    
    let local_addr = socket.local_addr()?;
    Ok(local_addr.ip())
}
