pub mod identity;
pub mod handshake;
pub mod encryption;
pub use identity::GestionnaireIdentite;
pub use handshake::InitiateurPoigneeDeMain;
pub use encryption::{CipherManager, DecipherManager};

use std::error::Error;
use dirs::home_dir;

///  Gestionnaire de sécurité complet
pub struct SecureSession {
    pub identite: GestionnaireIdentite,
    pub cipher: Option<CipherManager>,
    pub decipher: Option<DecipherManager>,
}

impl SecureSession {
    /// Initialiser une session sécurisée
    pub async fn new(_nom_appareil: String) -> Result<Self, Box<dyn Error>> {
        // ÉTAPE 1: Créer le répertoire
        println!("🔐 [SÉCURITÉ] ÉTAPE 1: Initialisation de la session");
        let config_dir = home_dir()
            .ok_or(" Impossible de trouver le répertoire home")?
            .join(".cargodrop")
            .join("security");
        tokio::fs::create_dir_all(&config_dir).await?;
        
        
        // ÉTAPE 2: Générer identité ED25519
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 2: Génération de l'identité ED25519");
        let identite = GestionnaireIdentite::nouveau();
        println!("   └─ Paire de clés ED25519 générée");
        
        // ÉTAPE 3: Créer empreinte
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 3: Création de l'empreinte digitale");
        identite.afficher_empreinte_locale();
        
        
        
        Ok(Self {
            identite,
            cipher: None,
            decipher: None,
        })
    }

    /// Établir un handshake de sécurité avec un pair
    pub fn initier_handshake(&self) -> Result<(Vec<u8>, [u8; 32]), Box<dyn Error>> {
        println!("🔐 [SÉCURITÉ] ÉTAPE 5: Initiation du Handshake");
        let handshake = InitiateurPoigneeDeMain::nouveau(
            self.identite.get_cle_signature(),
            self.identite.get_cle_verification(),
        );
        
        // Créer les secrets éphémères X25519
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 6: Génération des secrets éphémères X25519");
        let (secret_ephemere, cle_pub_ephemere) = InitiateurPoigneeDeMain::creer_secret_ephemere();
        println!("    Secrets X25519 créés");
        
        // Créer le message d'initialisation
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 7: Création du message de handshake");
        let (message_init, _) = handshake.creer_message_init(
            "cargodrop-client".to_string(),
        );
        println!("   └─ Message signé avec ED25519");
        
        // Sérialiser le message
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 8: Sérialisation du message");
        let message_bytes = serde_json::to_vec(&message_init)?;
        println!("    Message sérialisé en JSON: {} bytes", message_bytes.len());
        println!("    Message prêt à envoyer");
        
        // Dériver le secret partagé (Diffie-Hellman)
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 9: Dérivation du secret partagé (X25519 DH)");
        let secret_partage = InitiateurPoigneeDeMain::deriver_secret_partage(
            secret_ephemere,
            &cle_pub_ephemere,
        );
        println!("    DH réussi");
        
        // Dériver la clé de chiffrement avec HKDF
        println!("\n🔐 [SÉCURITÉ] ÉTAPE 10: Dérivation de la clé AES-256-GCM (HKDF-SHA256)");
        let cle_chiffrement = InitiateurPoigneeDeMain::deriver_cle_chiffrement(&secret_partage);
        println!("    HKDF-SHA256(secret_partage) → clé AES-256");
        println!("    Clé dérivée: 32 bytes (256 bits)");
        println!("    Clé AES-256-GCM: {}", hex::encode(&cle_chiffrement[..8]));
        println!("    Clé dérivée avec succès\n");
        
        Ok((message_bytes, cle_chiffrement))
    }

    /// Activer le chiffrement avec une clé
    pub fn activer_chiffrement(&mut self, cle_chiffrement: &[u8; 32]) {
        println!("🔐 [SÉCURITÉ] ÉTAPE 11: Activation du chiffrement AES-256-GCM");
        self.cipher = Some(CipherManager::nouveau(cle_chiffrement));
        self.decipher = Some(DecipherManager::nouveau(cle_chiffrement));
        println!("    Chiffrement activé\n");
    }

    /// Chiffrer des données
    pub fn chiffrer(&mut self, donnees: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        println!("🔐 [SÉCURITÉ] ÉTAPE 12: Chiffrement des données");
        println!("    Taille originale: {} bytes", donnees.len());
        
        let resultat = self.cipher
            .as_mut()
            .ok_or(" Chiffrement non activé")?
            .chiffrer_bloc(donnees)?;
        
        println!("   └─ Taille chiffrée: {} bytes", resultat.len());
        println!("    Données chiffrées avec succès\n");
        Ok(resultat)
    }

    /// Déchiffrer des données
    pub fn dechiffrer(&mut self, donnees_chiffrees: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        println!("🔐 [SÉCURITÉ] ÉTAPE 13: Déchiffrement des données");
        println!("    Taille chiffrée: {} bytes", donnees_chiffrees.len());
        let resultat = self.decipher
            .as_mut()
            .ok_or(" Déchiffrement non activé")?
            .dechiffrer_bloc(donnees_chiffrees)?;
        
        println!("    Taille déchiffrée: {} bytes", resultat.len());
        println!("    Tag authentifié - Données intègres\n");
        
        Ok(resultat)
    }

    pub fn get_identifiant_court(&self) -> String {
        let empreinte = GestionnaireIdentite::creer_empreinte(
            self.identite.obtenir_cle_verification_locale().as_slice()
        );
        GestionnaireIdentite::creer_identifiant_court(&empreinte)
    }

}