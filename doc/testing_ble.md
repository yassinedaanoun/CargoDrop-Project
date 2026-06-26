# Testing the CargoDrop BLE Rendezvous Service

This guide explains how to test the custom advertising payload introduced in the BLE rendezvous module.

## 1. Prerequisites
- A mobile phone or another device with a BLE scanner application. We recommend:
  - **nRF Connect for Mobile** (available on iOS and Android)
  - **LightBlue** (available on iOS and Android)
- Your Rust process running locally with the Bluetooth adapter powered on.

## 2. Starting the Advertiser
Run your application or directly execute the `advertise_rendezvous()` function. The console should print:
```
Encoded Network payload (IP: [192, 168, 1, 100], Port: 8080) -> Name: 'wKgBZB_g'
Ensuring Bluetooth adapter is powered on...
GATT Service added locally.
Now actively advertising custom network rendezvous info...
```

*(Note that `wKgBZB_g` or a similarly sized 8-character string will vary depending on your actual IP/Port config).*

## 3. Scanning and Reading Packets
Once the program is running:
1. Open your BLE scanner app (e.g., nRF Connect).
2. Start a fresh **Scan**. You might want to apply a filter for our Service UUID: `d59218d6-6b22-4a0b-9ba7-70e28148b488`.
3. Locate the device. Look for the **Device Name** in the advertising data.

## 4. Decoding the Payload
The device name serves as our compact 6-byte payload encoded in Base64 (URL-safe, no padding).

For example, if the device name is `wKgBZB_g`:
1. **Decode the Base64 String**:
   Base64 `wKgBZB_g` translates to 6 bytes: `[192, 168, 1, 100, 31, 224]`.
2. **Extract IPv4 Address**:
   The first 4 bytes `[192, 168, 1, 100]` map directly to the IPv4 address `192.168.1.100`.
3. **Extract Port**:
   The last 2 bytes `[31, 224]` map to a 16-bit integer (Network big-endian order).
   `31 * 256 + 224 = 8160` -> The HTTP server is port 8160.
   
*Clients can now use this IPv4 and Port to seamlessly switch from a BLE discovery connection to a direct TCP/HTTP network connection without needing explicit routing setup.*
