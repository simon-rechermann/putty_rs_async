use clap::{Parser}; // Bring in the derive macro
use crate::core::connection::Connection; // So we can call write(), read(), etc.
use crate::core::{ConnectionManager, ConnectionError};
use crate::connections::serial::SerialConnection;

/// Command-line arguments struct.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0")]
pub struct Cli {
    /// Serial port to open (e.g. /dev/ttyUSB0)
    #[arg(long)]
    pub port: Option<String>,
}

/// Main function that handles CLI parsing and runs the logic.
pub fn run_cli() -> Result<(), ConnectionError> {
    // Parse CLI arguments
    let cli = Cli::parse();

    if let Some(port) = cli.port {
        eprintln!("Opening serial port: {port}");
        let baud_rate = 115200;

        // Create a ConnectionManager and a SerialConnection
        let manager = ConnectionManager::new();
        let conn = SerialConnection::new(port.clone(), baud_rate);

        // Attempt to connect
        let mut active_conn = manager.create_connection(conn)?;

        // Write some data
        let data_to_write = b"Hello, serial world!\n";
        let bytes_written = active_conn.write(data_to_write)?;
        println!("Wrote {} bytes to {}", bytes_written, port);

        // Try reading some data (non-blocking, may fail if no data is available)
        let mut buffer = [0u8; 64];
        match active_conn.read(&mut buffer) {
            Ok(size) => {
                println!("Read {} bytes: {:?}", size, &buffer[..size]);
            }
            Err(e) => {
                println!("Read error (probably no incoming data): {:?}", e);
            }
        }

        // Disconnect
        manager.destroy_connection(active_conn)?;
    } else {
        eprintln!("No --port argument provided. Usage: putty_rs --port /dev/ttyUSB0");
    }

    Ok(())
}
