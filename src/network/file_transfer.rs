use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Information supplied by the discovery layer to connect to a peer.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub ip: String,
    pub port: u16,
    pub device_name: String,
}

/// Metadata sent before file bytes are streamed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHeader {
    pub filename: String,
    pub file_size: u64,
    pub file_type: String,
}

/// Handshake request sent by the sender before transfer starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub device_name: String,
    pub file_header: FileHeader,
}

/// Handshake response sent by the receiver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponse {
    pub device_name: String,
    pub accepted: bool,
    pub message: String,
}

/// Shared utilities for protocol framing, chunk transfer, and progress formatting.
pub struct FileTransfer;

impl FileTransfer {
    pub const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

    /// Prefixes a JSON payload with a u32 length and writes it to the stream.
    pub fn send_json_message<T: Serialize>(
        stream: &mut std::net::TcpStream,
        message: &T,
    ) -> Result<(), Box<dyn Error>> {
        let encoded = serde_json::to_vec(message)?;
        let len = u32::try_from(encoded.len())?;

        stream.write_all(&len.to_be_bytes())?;
        stream.write_all(&encoded)?;
        stream.flush()?;

        Ok(())
    }

    /// Reads a length-prefixed JSON payload from the stream.
    pub fn read_json_message<T: for<'de> Deserialize<'de>>(
        stream: &mut std::net::TcpStream,
    ) -> Result<T, Box<dyn Error>> {
        let mut len_bytes = [0_u8; 4];
        stream.read_exact(&mut len_bytes)?;
        let payload_len = u32::from_be_bytes(len_bytes) as usize;

        let mut payload = vec![0_u8; payload_len];
        stream.read_exact(&mut payload)?;

        let message = serde_json::from_slice::<T>(&payload)?;
        Ok(message)
    }

    /// Builds a metadata header from a local file path.
    pub fn build_file_header(file_path: &str) -> Result<FileHeader, Box<dyn Error>> {
        let path = Path::new(file_path);
        let metadata = std::fs::metadata(path)?;
        let filename = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or("Could not derive filename from path")?
            .to_string();

        Ok(FileHeader {
            filename,
            file_size: metadata.len(),
            file_type: Self::detect_file_type(path),
        })
    }

    /// Streams file bytes to the socket in fixed-size chunks and reports cumulative bytes sent.
    pub fn send_file_bytes(
        stream: &mut std::net::TcpStream,
        file_path: &str,
        progress_tx: Sender<u64>,
    ) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(file_path)?;
        let mut buffer = [0_u8; Self::CHUNK_SIZE];
        let mut sent = 0_u64;

        loop {
            let read_count = file.read(&mut buffer)?;
            if read_count == 0 {
                break;
            }

            stream.write_all(&buffer[..read_count])?;
            sent += read_count as u64;
            let _ = progress_tx.send(sent);
        }

        stream.flush()?;
        Ok(())
    }

    /// Reads exactly total_size bytes from the stream and writes them to output_file.
    pub fn receive_file_bytes(
        reader: &mut BufReader<std::net::TcpStream>,
        output_file: &mut File,
        total_size: u64,
        progress_tx: Sender<u64>,
    ) -> Result<(), Box<dyn Error>> {
        let mut remaining = total_size;
        let mut buffer = [0_u8; Self::CHUNK_SIZE];
        let mut received = 0_u64;

        while remaining > 0 {
            let to_read = std::cmp::min(remaining as usize, Self::CHUNK_SIZE);
            let bytes_read = reader.read(&mut buffer[..to_read])?;

            if bytes_read == 0 {
                return Err("Connection closed before file transfer completed".into());
            }

            output_file.write_all(&buffer[..bytes_read])?;
            received += bytes_read as u64;
            remaining -= bytes_read as u64;
            let _ = progress_tx.send(received);
        }

        output_file.flush()?;
        Ok(())
    }

    /// Returns a simple time marker for stdout logs.
    pub fn timestamp() -> String {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration.as_secs();
                format!("{}", seconds)
            }
            Err(_) => "0".to_string(),
        }
    }

    /// Human-readable byte formatter used by progress logs.
    pub fn human_bytes(bytes: u64) -> String {
        let value = bytes as f64;
        let kb = 1024.0;
        let mb = kb * 1024.0;
        let gb = mb * 1024.0;

        if value >= gb {
            format!("{:.2} GB", value / gb)
        } else if value >= mb {
            format!("{:.2} MB", value / mb)
        } else if value >= kb {
            format!("{:.2} KB", value / kb)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Returns transfer percentage in the [0, 100] range.
    pub fn percentage(done: u64, total: u64) -> f64 {
        if total == 0 {
            return 100.0;
        }
        (done as f64 / total as f64) * 100.0
    }

    fn detect_file_type(path: &Path) -> String {
        path.extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.to_lowercase())
            .filter(|ext| !ext.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    }
}