use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};
use sha2::{Sha256, Digest};
use hkdf::Hkdf;
use ed25519_dalek::{Signature, Signer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]



pub struct MessagePoigneeDeMainInit {
    pub cle_ephemere_pub: Vec<u8>,
    pub signature_ephemere: Vec<u8>,
    pub signature_message: Vec<u8>,      
    pub cle_identite: Vec<u8>,
    pub nom_appareil: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MessagePoigneeDeMainReponse {
    pub cle_ephemere_pub: Vec<u8>,
    pub signature_ephemere: Vec<u8>,
    pub signature_message: Vec<u8>,      
    pub cle_identite: Vec<u8>,
    pub nom_appareil: String,
    pub hmac_confirmation: Vec<u8>,
}

pub struct InitiateurPoigneeDeMain {
    cle_signature_locale: ed25519_dalek::SigningKey,
    cle_verification_locale: ed25519_dalek::VerifyingKey,
}

impl InitiateurPoigneeDeMain {
    pub fn nouveau(
        cle_signature: ed25519_dalek::SigningKey,
        cle_verification: ed25519_dalek::VerifyingKey,
    ) -> Self {
        Self {
            cle_signature_locale: cle_signature,
            cle_verification_locale: cle_verification,
        }
    }
    
    pub fn creer_secret_ephemere() -> (EphemeralSecret, X25519PublicKey) {
        let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let public = X25519PublicKey::from(&secret);
        (secret, public)
    }
    
    pub fn signer_cle_ephemere(
        &self,
        cle_pub_ephemere: &X25519PublicKey,
    ) -> Signature {
        self.cle_signature_locale.sign(cle_pub_ephemere.as_bytes())
    }
    
    pub fn creer_message_init(
        &self,
        nom_appareil: String,
    ) -> (MessagePoigneeDeMainInit, EphemeralSecret) {
        let (secret_ephemere, cle_pub_ephemere) = Self::creer_secret_ephemere();
        let signature_ephemere = self.signer_cle_ephemere(&cle_pub_ephemere);
        
        //  NOUVEAU: Construire le hash du message avant signature
        let mut hasher = Sha256::new();
        hasher.update(cle_pub_ephemere.as_bytes());
        hasher.update(nom_appareil.as_bytes());
        hasher.update(self.cle_verification_locale.as_bytes());
        let message_hash = hasher.finalize();
        
        //  Signer le hash complet du message
        let signature_message = self.cle_signature_locale.sign(&message_hash);
        
        let message = MessagePoigneeDeMainInit {
            cle_ephemere_pub: cle_pub_ephemere.as_bytes().to_vec(),
            signature_ephemere: signature_ephemere.to_bytes().to_vec(),
            signature_message: signature_message.to_bytes().to_vec(), 
            cle_identite: self.cle_verification_locale.as_bytes().to_vec(),
            nom_appareil,
        };
        
        (message, secret_ephemere)
    }
    
    pub fn deriver_secret_partage(
        secret_ephemere: EphemeralSecret,
        cle_pub_ephemere_pair: &X25519PublicKey,
    ) -> [u8; 32] {
        let secret_partage: SharedSecret = secret_ephemere.diffie_hellman(cle_pub_ephemere_pair);
        *secret_partage.as_bytes()
    }
    
    pub fn deriver_cle_chiffrement(secret_partage: &[u8; 32]) -> [u8; 32] {
        let hkdf = Hkdf::<Sha256>::new(None, secret_partage);
        let info = b"cargodrop-aes256-gcm";
        let mut cle_chiffrement = [0u8; 32];
        
        hkdf.expand(info, &mut cle_chiffrement)
            .expect("Erreur HKDF - la taille de sortie est valide");
        
        cle_chiffrement
    }
   
}