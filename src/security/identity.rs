use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};
use std::error::Error;

/// Représente l'identité cryptographique d'un pair
#[derive(Debug, Clone)]
pub struct IdentitePair {
    pub cle_publique: Vec<u8>,
    pub empreinte: String,
    pub nom_appareil: String,
}

/// Gère la génération et vérification des identités cryptographiques
#[derive(Clone)]
pub struct GestionnaireIdentite {
    cle_signature_locale: SigningKey,
    cle_verification_locale: VerifyingKey,
}

impl GestionnaireIdentite {
    pub fn nouveau() -> Self {
        let mut secret_bytes = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        
        let cle_signature = SigningKey::from_bytes(&secret_bytes);
        let cle_verification = cle_signature.verifying_key();
        Self {
            cle_signature_locale: cle_signature,
            cle_verification_locale: cle_verification,
        }
    }
    
   
    
    pub fn obtenir_cle_verification_locale(&self) -> Vec<u8> {
        self.cle_verification_locale.as_bytes().to_vec()
    }
    
    pub fn creer_empreinte(cle_publique: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(cle_publique);
        let hash = hasher.finalize();
        
        format!("{:x}", hash)[..16].to_string()
    }
    
    pub fn creer_identite_locale(&self, nom_appareil: String) -> IdentitePair {
        let cle_pub_bytes = self.obtenir_cle_verification_locale();
        let empreinte = Self::creer_empreinte(&cle_pub_bytes);
        
        IdentitePair {
            cle_publique: cle_pub_bytes,
            empreinte,
            nom_appareil,
        }
    }
    
    pub fn signer(&self, donnees: &[u8]) -> Signature {
        
        self.cle_signature_locale.sign(donnees)
    }
    
    pub fn verifier_signature(
        cle_publique_pair: &[u8],
        donnees: &[u8],
        signature_bytes: &[u8; 64],
    ) -> Result<(), Box<dyn Error>> {
        let cle_verification = VerifyingKey::from_bytes(
            <&[u8; 32]>::try_from(&cle_publique_pair[..32])?
        )?;
        
        let signature = Signature::from_bytes(signature_bytes);
        cle_verification.verify_strict(donnees, &signature)?;
        
        Ok(())
    }
    
    pub fn creer_identifiant_court(empreinte: &str) -> String {
        // Prendre les 4 premiers caractères de l'empreinte
        empreinte[..4].to_string()
    }

    pub fn afficher_empreinte_locale(&self) {
        let empreinte = Self::creer_empreinte(self.obtenir_cle_verification_locale().as_slice());
        println!("\n╔════════════════════════════════════════╗");
        println!("║     VOTRE EMPREINTE DE SÉCURITÉ        ║");
        println!("║                                        ║");
        println!("║  Partagez ce code avec vos collègues   ║");
        println!("║  pour vérifier votre identité          ║");
        println!("║                                        ║");
        println!("║         {}         ║", empreinte);
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝\n");
    }
    
    pub fn get_cle_signature(&self) -> SigningKey {
        self.cle_signature_locale.clone()
    }
    
    pub fn get_cle_verification(&self) -> VerifyingKey {
        self.cle_verification_locale.clone()
    }
}