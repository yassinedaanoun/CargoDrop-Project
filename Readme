# CargoDrop-Project
Outil P2P décentralisé en Rust pour le transfert sécurisé de fichiers sur LAN. Architecture Zero-Trust : authentification mutuelle Ed25519, échange de clés éphémères X25519 , chiffrement AES-256-GCM et protections durcies contre les attaques de type MITM et DoS/OOM.


CargoDrop est un outil de transfert de fichiers Peer-to-Peer (P2P) décentralisé, développé en Rust, conçu spécifiquement pour garantir une confidentialité et une intégrité absolues lors des échanges sur réseau local (LAN). 

L'application implémente une architecture Zero-Trust, inspirée des protocoles de messagerie sécurisée modernes , garantissant qu'aucune donnée ne transite en clair et qu'aucun pair ne puisse usurper une identité.

---

##  Architecture de Sécurité (Zéro-Trust)

Le protocole de sécurité de CargoDrop est segmenté en 4 phases strictes et transparentes, conçues pour neutraliser les attaques passives (écoute) et actives (Man-In-The-Middle, Déni de Service) :

### Phase 1 : Identité des Pairs & Authentification Mutuelle
* **Algorithme :** Ed25519 (Signature sur courbe elliptique).
* **Mécanisme :** Chaque nœud génère son propre couple de clés asymétriques. L'identité est validée de manière décentralisée : une empreinte cryptographique courte (SHA-256) est affichée à l'écran et validée visuellement ou vocalement par les utilisateurs. Une fois approuvé, le pair est inscrit dans un magasin de confiance local (Liste VIP).

### Phase 2 : Échange de Clés & Forward Secrecy
* **Algorithme :** X25519 (ECDH éphémère) + HKDF-SHA256.
* **Mécanisme :** Pour chaque session de transfert, les pairs génèrent des clés Diffie-Hellman éphémères signées numériquement par leur clé d'identité (Ed25519). Un secret partagé est calculé puis étendu via HKDF pour dériver une clé de chiffrement unique à la session. Ce mécanisme garantit la Forward Secrecy (la compromission future d'une clé d'identité ne permet pas de déchiffrer les sessions passées). Un HMAC de confirmation valide l'alignement des clés avant tout envoi de fichier.

### Phase 3 : Transport Chiffré & Authentifié
* **Algorithme :** AES-256-GCM (Authenticated Encryption with Associated Data).
* **Mécanisme :** Les flux de données TCP sont segmentés en blocs. Chaque bloc est chiffré individuellement avec AES-256-GCM. L'utilisation du mode GCM fournit un tag d'authentification (MAC) garantissant qu'aucun octet n'a été altéré ou injecté durant le transport.

### Phase 4 : Résilience Réseau & Durcissement (Anti-DoS)
* **Gestion des Nonces :** Intégration de nonces séquentiels et incrémentaux au sein du gestionnaire de déchiffrement. Si un bloc arrive hors d'ordre ou est rejoué, la session est immédiatement avortée.
* **Atténuation DoS/OOM :** Le protocole réseau applique une validation stricte de la taille des buffers avant toute allocation dynamique de mémoire (`MAX_BLOC_SIZE`). Si un attaquant transmet un paquet malveillant annonçant une taille de bloc disproportionnée, la tentative est bloquée en amont, empêchant le crash de l'application par épuisement de mémoire (Out-Of-Memory).

---

##  Workflow Réseau

1. **Découverte (BLE Rendezvous) :** Utilisation du protocole Bluetooth Low Energy (BLE) pour la découverte rapide des pairs et la signalisation des métadonnées réseau (IP/Port).
2. **Handshake Cryptographique :** Établissement de la connexion TCP, validation mutuelle des identités Ed25519 et dérivation du secret éphémère X25519.
3. **Signalisation de Transfert :** Envoi de la requête de métadonnées du fichier (`TransferRequest`) uniquement après sécurisation du tunnel.
4. **Streaming Chiffré :** Découpage du fichier en blocs de 64 Ko, chiffrement AES-GCM à la volée et écriture sécurisée côté récepteur.

---

##  Stack Technique

* **Langage :** Rust (garantie de la sécurité mémoire sans garbage collector)
* **Cryptographie :** `aes-gcm`, `ed25519-dalek`, `x25519-dalek`, `hkdf`, `sha2`, `hmac`
* **Réseau :** Standard TCP (`std::net::TcpStream`/`TcpListener`), BLE (`ble-peripheral-rust`)
* **Sérialisation :** `serde`, `serde_json`

---

##  Structure du Code Source

```text
cargodrop/
├── src/
│   ├── security/
│   │   ├── identity.rs     # Génération Ed25519, empreintes et dialogues d'approbation
│   │   ├── handshake.rs    # Protocole ECDH X25519, HKDF et HMAC de confirmation
│   │   ├── encryption.rs   # Chiffrement AES-256-GCM et gestionnaire de nonces séquentiels
│   │   └── contacts.rs     # Gestion persistante de la liste des contacts de confiance
│   ├── network/
│   │   ├── tcp_client.rs   # Logique client d'initialisation de poignée de main et envoi chiffré
│   │   ├── tcp_server.rs   # Serveur multithreadé durci avec vérifications anti-DoS
│   │   └── file_transfer.rs# Structures de signalisation et gestion des fichiers
│   └── main.rs             # CLI d'interface utilisateur (Advertise / Discover / Receive)
