use std::error::Error;
use std::net::TcpStream;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use crate::network::file_transfer::{FileTransfer, PeerInfo, TransferRequest, TransferResponse};
use crate::ui::interaction::InteractionHandler;

/// TCP sender that connects to a peer and streams one file.
pub struct TcpClient {
    peer: PeerInfo,
    local_device_name: String,
    handler: Arc<dyn InteractionHandler>,
}

impl TcpClient {
    /// Creates a new sender client.
    pub fn new(
        peer: PeerInfo,
        local_device_name: String,
        handler: Arc<dyn InteractionHandler>,
    ) -> Self {
        Self {
            peer,
            local_device_name,
            handler,
        }
    }

    /// Connects to the peer, performs handshake, and sends the file bytes.
    pub fn send_file(&self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let address = format!("{}:{}", self.peer.ip, self.peer.port);
        println!(
            "[{}] Connecting to {} ({})",
            FileTransfer::timestamp(),
            address,
            self.peer.device_name
        );

        let mut stream = TcpStream::connect(&address)?;
        let file_header = FileTransfer::build_file_header(file_path)?;

        let request = TransferRequest {
            device_name: self.local_device_name.clone(),
            file_header: file_header.clone(),
        };
        FileTransfer::send_json_message(&mut stream, &request)?;

        let response: TransferResponse = FileTransfer::read_json_message(&mut stream)?;
        if !response.accepted {
            return Err(format!("Transfer refused by peer: {}", response.message).into());
        }

        println!(
            "[{}] Handshake accepted by '{}' - sending '{}' ({}).",
            FileTransfer::timestamp(),
            response.device_name,
            file_header.filename,
            FileTransfer::human_bytes(file_header.file_size)
        );

        let total_size = file_header.file_size;
        let start_time = Instant::now();
        let (progress_tx, progress_rx) = mpsc::channel::<u64>();
        let handler = self.handler.clone();

        let progress_thread = thread::spawn(move || {
            while let Ok(done) = progress_rx.recv() {
                handler.update_progress("Sending...", done, total_size);
            }
        });

        FileTransfer::send_file_bytes(&mut stream, file_path, progress_tx)?;

        if let Err(err) = progress_thread.join() {
            eprintln!(
                "[{}] Progress thread ended unexpectedly: {:?}",
                FileTransfer::timestamp(),
                err
            );
        }

        let elapsed = start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        let speed_mbs = (total_size as f64 / (1024.0 * 1024.0)) / elapsed_secs;
        
        println!(
            "[{}] File sent successfully in {:.2}s at {:.2} MB/s.",
            FileTransfer::timestamp(),
            elapsed_secs,
            speed_mbs
        );
        Ok(())
    }
}