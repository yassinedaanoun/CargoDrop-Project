# Network Protocol Recap: QUIC in CargoDrop

This report summarizes the developer choices for the network protocols used in the **CargoDrop** airdrop application, focusing on the selection of the QUIC protocol.

## 1. What is QUIC?
QUIC (Quick UDP Internet Connections) is a modern transport layer network protocol designed by Google and now standardized by the IETF. It sits on top of **UDP** rather than TCP.

Unlike the traditional TCP+TLS stack, QUIC integrates security and transport into a single handshake, making it significantly faster and more robust for modern applications.

## 2. Security & Speed Requirements

To ensure CargoDrop is both secure and high-performance, QUIC requires several implementation strategies:

### Security
- **Mandatory TLS 1.3**: QUIC cannot run without encryption. It uses TLS 1.3 by default, ensuring all data is encrypted from the very first byte.
- **Self-Signed Certificates**: In a peer-to-peer (P2P) context like CargoDrop, we generate on-the-fly self-signed certificates to establish identity.
- **Authenticated Handshake**: We implement an additional **Diffie-Hellman (X25519)** key exchange and a 6-digit confirmation code to prevent Man-in-the-Middle (MITM) attacks.

### Speed
- **0-RTT / 1-RTT Handshakes**: QUIC reduces the number of round-trips needed to start a connection. If two devices have talked before, they can start sending data immediately (0-RTT).
- **Stream Multiplexing**: Multiple files or messages can be sent over different "streams" within a single connection.
- **Head-of-Line Blocking Removal**: If one packet is lost in one stream, other streams can continue processing data (unlike TCP, where everything stops until the lost packet is retransmitted).

## 3. Why choose QUIC for CargoDrop?

The choice of QUIC was driven by the specific needs of an airdrop application:

1.  **Peer-to-Peer Stability**: QUIC handles **Connection Migration**. If your device switches from Wi-Fi to 5G (or the signal flickers), the connection stays alive because it's not tied to a specific IP/Port pair but to a Connection ID.
2.  **Fast Transfers**: High-speed file transfers are critical. QUIC's optimized congestion control and lack of head-of-line blocking maximize throughput on unreliable local networks.
3.  **Security by Design**: Since encryption is non-optional, we don't have to worry about "unprotected" modes; the application is secure by default.

## 4. Pros and Cons of Our Approach

| Feature | Pros | Cons |
| :--- | :--- | :--- |
| **QUIC Protocol** | Extreme speed, resilient to packet loss, built-in encryption, connection migration. | More complex to implement than simple TCP/UDP; higher CPU overhead for encryption. |
| **Self-Signed Certs** | No need for a central Authority (CA), perfect for offline/local P2P. | Requires manual verification (code) to ensure the peer is who they claim to be. |
| **Rust (Quinn)** | Safe, concurrent, and high-performance implementation. | Rust's strictness can slow down initial development of complex network state machines. |

## 5. Summary
QUIC provides the perfect balance of **security** (always encrypted) and **performance** (no latency bottlenecks). For CargoDrop, it ensures that file transfers are as fast as the hardware allows while protecting user privacy through modern cryptographic standards.
