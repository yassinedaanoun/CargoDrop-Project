# CargoDrop Rendezvous Component Recap

## Purpose of Rendezvous

The primary goal is to enable client devices to **discover each other on the network**. 
Before any file transfer can occur, devices must:
- **Gather network contact info:** Collect IP addresses and ports, to later establish a peer-to-peer connection for data transport.
- **Exchange identification:** Find out names of nearby users.

Meaning that this component must be able to 
- advertise our presence to others
- detect the others
- keep a live stream of "detected peers"
- manage disconnections

## Rendezvous Methods

### Preferred Method: LAN (Local Area Network)

The most efficient way to discover peers is over a shared local network, because of the fast transfer speeds. 

To do that discovery, we do something called **Service Discovery (DNS-SD):** 

We define an identifier for our app that will define the *service* the peer devices will provide (like `_rustdrop._tcp.local`). Anyone on the LAN that'll ask for people with the defined service will get answers from the other peers, and will provide him their "contact info" (ip address and other) to communicate.

This service discovery is based on **mDNS**, mutlicast Domain Name Server, an architecture that allow each device to *broadcast* themselves over the local network.

**Unfortunatelyn mDNS is blocked over all public WiFi networks. For those, we will need to find another mean of detection.**

### Fallback Method: Bluetooth (BLE)

In environments where LAN discovery fails or is unavailable, Bluetooth Low Energy (BLE) serves as a fallback. But it is not fast enough for file trasnfers, so we will keep it as pure discovery.

*How it works:* 
All bluetooth architectures work in 2 distinct modes, **Central** and **Peripheral**.
A *peripheral* advertises its presence to others, while a *central* listens to others' advertisements (scanner). In the case of our P2P app, we will be **both**.

We will have a similar approach to the **DNS-SD** : we will create a unique identifier (UUID), and retrieve the device network info (ip address...).

Then we pack all that data into an small **Advertising Packet**, that we will broadcast multiple times per second using Bluetooth (no wifi). We scan for those packets when launching a `discover`, and extract the network info from there. 

> [!NOTE]
>  **Connection-less:** Unlike other bluetooth apps, our use case does not require formal bluetooth pairing between peers, since we'll still make use of the LAN to do the file transfers. Which is why we chose to avoid the trouble, and keep the gimmicky ADV packets to broadcast network information.

## Overview


| Feature         | LAN (mDNS)                | Bluetooth (BLE)                |
| :-------------- | :------------------------ | :----------------------------- |
| **Speed**       | Extremely Fast            | Slower Detection               |
| **Reliability** | Depends on Router/Network | Highly Reliable (Local)        |
| **Complexity**  | Standard / Easy           | Gimmicky / Custom              |
| **Data Rate**   | High (for info exchange)  | Very Low (limited packet size) |

> [!NOTE]
> **Hybrid Approach:** The final version will aim to use a hybrid model,searching on both LAN and BLE simultaneously to ensure maximum device visibility.

### Implementation Notes: The "Gimmicky" BLE
The BLE implementation relies heavily on external Rust crates. To maintain **multi-platform compatibility** (Linux, Windows, macOS), we avoided modifying the source code of these crates. This choice resulted in a slightly "gimmicky" setup where we store the payload of the ADV packet (the ip address and other) as the `device name` of the packet, instead of putting it where it belongs (manufacturer info ...).

### Why a "Wi-Fi-less" CargoDrop is Not Feasible
A common question is why CargoDrop doesn't implement a fully "Wi-Fi-less" experience similar to Apple's AirDrop (which uses Wi-Fi Direct/AWDL alongside BLE).

CargoDrop requires a common wifi (but not internet) connection between peers for the following reason:

In other production grade implementations (Airdrop, QuickShare), what happens in the background is the creation of an ad-hoc invisible wifi connection between peers at the transfer time (*Wifi-Direct*, or *AWDL* on MacOS). 

Creating such ad-hoc connections is an immense undertaking, as there is no current open-source stable solution to this. Wi-Fi Direct APIs are handled very differently across Linux, Windows, and macOS. There is no common, reliable abstraction layer that works seamlessly everywhere.


