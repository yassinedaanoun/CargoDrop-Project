use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use crate::network::file_transfer::{FileTransfer, TransferRequest, TransferResponse};
use crate::ui::interaction::InteractionHandler;

/// TCP receiver that accepts incoming file transfers.
pub struct TcpServer {
    bind_port: u16,
    device_name: String,
    handler: Arc<dyn InteractionHandler>,
}

impl TcpServer {
    /// Creates a new TCP server bound to 0.0.0.0:port.
    pub fn new(port: u16, device_name: String, handler: Arc<dyn InteractionHandler>) -> Self {
        Self {
            bind_port: port,
            device_name,
            handler,
        }
    }

    /// Starts listening and spawns one thread per incoming connection.
    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        let address = format!("0.0.0.0:{}", self.bind_port);
        let listener = TcpListener::bind(&address)?;

        println!(
            "[{}] Receiver listening on {}",
            FileTransfer::timestamp(),
            address
        );

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let device_name = self.device_name.clone();
                    let handler = self.handler.clone();
                    thread::spawn(move || {
                        if let Err(err) = Self::handle_connection(stream, device_name, handler) {
                            eprintln!("[{}] Connection error: {}", FileTransfer::timestamp(), err);
                        }
                    });
                }
                Err(err) => {
                    eprintln!("[{}] Accept error: {}", FileTransfer::timestamp(), err);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(
        mut stream: TcpStream,
        device_name: String,
        handler: Arc<dyn InteractionHandler>,
    ) -> Result<(), Box<dyn Error>> {
        let peer_addr = stream.peer_addr()?;
        println!(
            "[{}] Incoming connection from {}",
            FileTransfer::timestamp(),
            peer_addr
        );

        let request: TransferRequest = FileTransfer::read_json_message(&mut stream)?;
        println!(
            "[{}] Handshake from '{}' for file '{}' ({}).",
            FileTransfer::timestamp(),
            request.device_name,
            request.file_header.filename,
            FileTransfer::human_bytes(request.file_header.file_size)
        );

        let accepted = Self::confirm_transfer(&request)?;

        let response = TransferResponse {
            device_name,
            accepted,
            message: if accepted {
                "Ready to receive".to_string()
            } else {
                "Transfer rejected by user".to_string()
            },
        };
        FileTransfer::send_json_message(&mut stream, &response)?;

        if !accepted {
            println!(
                "[{}] Transfer refused for '{}' from '{}'.",
                FileTransfer::timestamp(),
                request.file_header.filename,
                request.device_name
            );
            return Ok(());
        }

        let download_dir = dirs::download_dir().unwrap_or_else(|| {
            eprintln!(
                "[{}] Warning: Downloads directory not found, using 'received' instead.",
                FileTransfer::timestamp()
            );
            PathBuf::from("received")
        });

        std::fs::create_dir_all(&download_dir)?;
        let output_path = download_dir.join(&request.file_header.filename);
        let mut output_file = File::create(&output_path)?;

        let total_size = request.file_header.file_size;
        let start_time = Instant::now();
        let (progress_tx, progress_rx) = mpsc::channel::<u64>();
        let handler_clone = handler.clone();

        let progress_thread = thread::spawn(move || {
            while let Ok(done) = progress_rx.recv() {
                handler_clone.update_progress("Receiving...", done, total_size);
            }
        });

        let mut reader = BufReader::new(stream);
        FileTransfer::receive_file_bytes(&mut reader, &mut output_file, total_size, progress_tx)?;

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
            "[{}] File received successfully in {:.2}s at {:.2} MB/s and saved to '{}'.",
            FileTransfer::timestamp(),
            elapsed_secs,
            speed_mbs,
            output_path.display()
        );

        Ok(())
    }

    fn confirm_transfer(request: &TransferRequest) -> Result<bool, Box<dyn Error>> {
        println!();
        println!("Incoming file transfer request:");
        println!("  Sender: {}", request.device_name);
        println!("  File: {}", request.file_header.filename);
        println!(
            "  Size: {}",
            FileTransfer::human_bytes(request.file_header.file_size)
        );
        println!("Accept transfer? [y/N]");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_ascii_lowercase();

        Ok(matches!(choice.as_str(), "y" | "yes"))
    }
}