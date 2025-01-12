use clap::Parser;
use log::{info, error};

use crate::core::{ConnectionManager, ConnectionError};
use crate::connections::serial::SerialConnection;
use crate::core::application::SerialSession;

/// Command-line arguments struct.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0")]
pub struct Cli {
    #[arg(long)]
    pub port: Option<String>,
    #[arg(long, default_value_t = 115200)]
    pub baud: u32,
}

pub fn run_cli() -> Result<(), ConnectionError> {
    let cli = Cli::parse();

    if let Some(port) = cli.port {
        eprintln!("Opening serial port: {} at {} baud", port, cli.baud);

        // Prepare manager and connection
        let manager = ConnectionManager::new();
        let connection = SerialConnection::new(port.clone(), cli.baud);
        // Must Box the connection if we want to store it as a dyn Connection
        let connection = manager.create_connection(connection)?;

        // Build a session
        let mut session = SerialSession::new(manager, Box::new(connection));

        // Run the session (reader thread + main loop)
        match session.run() {
            Ok(_) => info!("Session ended gracefully."),
            Err(e) => error!("Session error: {:?}", e),
        }
    } else {
        eprintln!("No --port argument provided.");
        eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
        eprintln!("Or usage: cargo run -- --port /dev/pts/3 --baud 115200");
    }

    Ok(())
}
