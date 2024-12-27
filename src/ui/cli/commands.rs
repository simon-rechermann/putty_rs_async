use clap::Parser; // We need the derive macro
use crate::core::connection::Connection; // So we can call write(), read(), etc.
use crate::core::{ConnectionManager, ConnectionError};
use crate::connections::serial::SerialConnection;
use std::io::{self, Write};

/// Command-line arguments struct.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0")]
pub struct Cli {
    /// Serial port to open (e.g. /dev/ttyUSB0)
    #[arg(long)]
    pub port: Option<String>,

    /// Baud rate (e.g. 9600, 115200)
    #[arg(long, default_value_t = 115200)]
    pub baud: u32,
}

/// Main function that handles CLI parsing and runs the logic.
pub fn run_cli() -> Result<(), ConnectionError> {
    // Parse CLI arguments
    let cli = Cli::parse();

    if let Some(port) = cli.port {
        eprintln!("Opening serial port: {} at {} baud", port, cli.baud);

        // Create a ConnectionManager and a SerialConnection
        let manager = ConnectionManager::new();
        let conn = SerialConnection::new(port.clone(), cli.baud);

        // Attempt to connect
        let mut active_conn = manager.create_connection(conn)?;

        //
        // Spawn a thread to continuously read from the serial port
        // and print incoming data to stdout.
        //
        // We send a clone of `active_conn`? Actually, we can't easily clone it
        // since it owns the port. Instead, we can use a shared reference
        // behind a Mutex or we do it more simply by **moving** the connection
        // into the thread. But then we can't also write from the main thread.
        //
        // A typical solution is to split read/write with e.g. `serialport::SerialPort::try_clone()`,
        // but many drivers do not fully support that. For a minimal approach, we'll do
        // a separate read thread that just polls the connection. Meanwhile, the main thread writes.
        //
        // We'll wrap active_conn in an Arc<Mutex<...>> so both threads can borrow it.
        use std::sync::{Arc, Mutex};
        let active_conn = Arc::new(Mutex::new(active_conn));

        // Create a channel to signal the read thread to stop
        let (tx_stop, rx_stop) = std::sync::mpsc::channel::<()>();

        // Clone the Arc for the reader thread
        let conn_reader = Arc::clone(&active_conn);

        std::thread::spawn(move || {
            let mut buffer = [0u8; 64];
            loop {
                // Check if we’ve been told to stop
                if let Ok(()) = rx_stop.try_recv() {
                    // Channel closed or stop message received
                    break;
                }
                {
                    let mut conn_guard = conn_reader.lock().unwrap();
                    match conn_guard.read(&mut buffer) {
                        Ok(size) if size > 0 => {
                            // Convert bytes to a string (lossy for safety)
                            let s = String::from_utf8_lossy(&buffer[..size]);
                            print!("{}", s);
                            io::stdout().flush().ok();
                        }
                        Ok(_) => {
                            // size = 0 means no data
                        }
                        Err(_) => {
                            // Could be a non-fatal read error. Let's just ignore or break
                            // but often you'd handle timeouts vs. real errors differently
                        }
                    }
                }

                // Sleep for a short time so we don't spin CPU at 100%
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        //
        // Main thread: read lines from stdin and write to serial port.
        //
        eprintln!("Type into this terminal to send to the serial port.");
        eprintln!("Type 'exit' (without quotes) to quit.");

        loop {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                // If user typed "exit", break
                if input.trim() == "exit" {
                    println!("Exiting loop...");
                    break;
                }

                // Otherwise, write to serial
                let mut conn_guard = active_conn.lock().unwrap();
                match conn_guard.write(input.as_bytes()) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Write error: {:?}", e);
                        break;
                    }
                }
            }
        }

        // Tell the reader thread to stop
        let _ = tx_stop.send(());
        // Optionally wait for the thread to join:
        // (we didn't keep its handle, so can't do join() directly)

        // Disconnect
        manager.destroy_connection(Arc::try_unwrap(active_conn).unwrap().into_inner().unwrap())?;
    } else {
        eprintln!("No --port argument provided.");
        eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
    }

    Ok(())
}
