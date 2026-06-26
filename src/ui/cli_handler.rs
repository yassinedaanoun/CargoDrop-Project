use crate::network::file_transfer::{FileTransfer, PeerInfo};
use crate::rendezvous::Peer;
use crate::ui::interaction::{InteractionHandler, PeerEvent};
use std::collections::HashMap;
use std::io::{self, Write};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const BLUE: &str = "\x1b[34m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn blue(s: &str) -> String {
    format!("{}{}{}", BLUE, s, RESET)
}

fn red(s: &str) -> String {
    format!("{}{}{}", RED, s, RESET)
}

pub struct CliHandler {
    last_table_height: AtomicUsize,
    last_events: Mutex<Vec<String>>,
}

impl CliHandler {
    pub fn new() -> Self {
        Self {
            last_table_height: AtomicUsize::new(0),
            last_events: Mutex::new(Vec::new()),
        }
    }
}

impl InteractionHandler for CliHandler {
    fn display_peers_list(&self, active_peers: &HashMap<String, Peer>, lost_peers: &HashMap<String, Peer>) {
        let b = |s: &str| blue(s);
        let r = |s: &str| red(s);

        // --- IN-PLACE UPDATE LOGIC ---
        // 1. Move cursor up by the height of the LAST table (if there was one)
        let prev_h = self.last_table_height.load(Ordering::Relaxed);
        if prev_h > 0 {
            // \x1b[{}A = Move cursor Up by N lines
            // \x1b[J   = Erase from cursor to end of screen
            print!("\x1b[{}A\x1b[J", prev_h);
        }

        let mut current_h = 0;
        let mut println_count = |s: &str| {
            println!("{}", s);
            current_h += 1;
        };

        // 2. Print all events occurred since last update above the table
        let mut events = self.last_events.lock().unwrap();
        for msg in events.drain(..) {
            println_count(&msg);
        }
        drop(events);

        // --- THE TABLE ---
        // Column widths
        let w_user = 19;
        let w_ip = 19;
        let w_port = 10;
        let total_w = w_user + w_ip + w_port + 2; // +2 for those internal pipes ┬/┼/┴

        let top = format!("{}{}{}{}{}{}{}", b("┌"), b(&"─".repeat(w_user)), b("┬"), b(&"─".repeat(w_ip)), b("┬"), b(&"─".repeat(w_port)), b("┐"));
        let mid = format!("{}{}{}{}{}{}{}", b("├"), b(&"─".repeat(w_user)), b("┼"), b(&"─".repeat(w_ip)), b("┼"), b(&"─".repeat(w_port)), b("┤"));
        let bot = format!("{}{}{}{}{}{}{}", b("└"), b(&"─".repeat(w_user)), b("┴"), b(&"─".repeat(w_ip)), b("┴"), b(&"─".repeat(w_port)), b("┘"));

        if active_peers.is_empty() && lost_peers.is_empty() {
            println_count(&top);
            println_count(&format!(
                "{}{:^w_user$}{}{:^w_ip$}{}{:^w_port$}{}",
                b("│"), "Username", b("│"), "IP Address", b("│"), "Port", b("│")
            ));
            println_count(&mid);
            println_count(&format!(
                "{}{:^total_w$}{}",
                b("│"), "No peers discovered yet.", b("│")
            ));
            println_count(&format!("{}\n", bot));
            io::stdout().flush().ok();
            self.last_table_height.store(current_h, Ordering::SeqCst);
            return;
        }

        println_count(&top);
        println_count(&format!(
            "{}{:^w_user$}{}{:^w_ip$}{}{:^w_port$}{}",
            b("│"), "Username", b("│"), "IP Address", b("│"), "Port", b("│")
        ));
        println_count(&mid);

        // Display Active Peers
        for peer in active_peers.values() {
            let ip_str = format!(
                "{}.{}.{}.{}",
                peer.ip[0], peer.ip[1], peer.ip[2], peer.ip[3]
            );
            println_count(&format!(
                "{}{:^w_user$}{}{:^w_ip$}{}{:^w_port$}{}",
                b("│"), peer.username, b("│"), ip_str, b("│"), peer.port, b("│")
            ));
        }

        // Display Lost Peers if any
        if !lost_peers.is_empty() {
            println_count(&mid);
            let lost_header = format!("{:^total_w$}", "--- X LOST PEERS ---");
            println_count(&format!("{}{}{}", b("│"), r(&lost_header), b("│")));
            println_count(&mid);
            for peer in lost_peers.values() {
                let ip_str = format!(
                    "{}.{}.{}.{}",
                    peer.ip[0], peer.ip[1], peer.ip[2], peer.ip[3]
                );
                println_count(&format!(
                    "{}{}{}{}{}{}{}",
                    b("│"),
                    r(&format!("{:^w_user$}", peer.username)),
                    b("│"),
                    r(&format!("{:^w_ip$}", ip_str)),
                    b("│"),
                    r(&format!("{:^w_port$}", peer.port)),
                    b("│")
                ));
            }
        }

        println_count(&format!("{}\n", bot));
        io::stdout().flush().ok();

        // Store the height for the next update move-up
        self.last_table_height.store(current_h, Ordering::SeqCst);
    }

    fn handle_peer_event(&self, event: PeerEvent) {
        let msg = match event {
            PeerEvent::NewPeer(peer, time) => {
                format!("\n[{}] --- 📡 PEER DETECTED: {} ---", time, peer.username)
            }
            PeerEvent::PeerLost(peer, time) => {
                format!("\n[{}] --- ❌ PEER DISCONNECTED: {} ---", time, peer.username)
            }
        };
        self.last_events.lock().unwrap().push(msg);
    }

    fn select_peer(&self, peers: &[PeerInfo]) -> Option<PeerInfo> {
        if peers.is_empty() {
            println!("No peers available to select.");
            return None;
        }

        println!("\n--- Select a Peer to Send File ---");
        for (i, peer) in peers.iter().enumerate() {
            println!(
                "{}. {} ({}:{})",
                i + 1,
                peer.device_name,
                peer.ip,
                peer.port
            );
        }

        print!("\nEnter peer number (or 'q' to cancel): ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        if input.to_lowercase() == "q" {
            return None;
        }

        if let Ok(choice) = input.parse::<usize>() {
            if choice > 0 && choice <= peers.len() {
                return Some(peers[choice - 1].clone());
            }
        }

        println!("Invalid selection.");
        None
    }

    fn on_advertising_start(&self, username: &str, ip: [u8; 4], port: u16, device_name_payload: &str) {
        let time_str = chrono::Local::now().format("%H:%M:%S").to_string();
        println!("[{}] --- ADVERTISING ACTIVE ---", time_str);
        println!(
            "  Broadcasting Username: '{}', IP: {}.{}.{}.{}, Port: {} inside Base64 Name: '{}'",
            username, ip[0], ip[1], ip[2], ip[3], port, device_name_payload
        );
    }

    fn update_progress(&self, message: &str, done: u64, total: u64) {
        let percent = FileTransfer::percentage(done, total);
        let bar_width = 25;
        let filled_width = (percent / 100.0 * bar_width as f64) as usize;
        let empty_width = bar_width - filled_width;
        let bar = format!(
            "[{}{}]",
            "=".repeat(filled_width),
            "-".repeat(empty_width)
        );

        print!(
            "\r[{}] {} {:>3.0}% {} {} / {}",
            FileTransfer::timestamp(),
            message,
            percent,
            bar,
            FileTransfer::human_bytes(done),
            FileTransfer::human_bytes(total)
        );
        io::stdout().flush().ok();

        if done == total {
            println!();
        }
    }
}
