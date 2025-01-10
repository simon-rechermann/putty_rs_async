use clap::Parser;
use crate::core::connection::Connection;
use crate::core::{ConnectionManager, ConnectionError};
use crate::connections::serial::SerialConnection;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use termios::*;

fn set_raw_mode() -> Termios {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(stdin_fd).unwrap();

    let original_termios = termios.clone();
    termios.c_lflag &= !(ICANON | ECHO); // Disable canonical mode and echo
    termios.c_cc[VMIN] = 1; // Minimum number of characters for a read
    termios.c_cc[VTIME] = 0; // Timeout in deciseconds

    tcsetattr(stdin_fd, TCSANOW, &termios).unwrap();
    original_termios
}

fn restore_mode(original: Termios) {
    let stdin_fd = io::stdin().as_raw_fd();
    tcsetattr(stdin_fd, TCSANOW, &original).unwrap();
}

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

        let manager = ConnectionManager::new();
        let conn = SerialConnection::new(port.clone(), cli.baud);

        let active_conn = manager.create_connection(conn)?;
        use std::sync::{Arc, Mutex};
        let active_conn = Arc::new(Mutex::new(active_conn));

        let (tx_stop, rx_stop) = std::sync::mpsc::channel::<()>();
        let conn_reader = Arc::clone(&active_conn);

        std::thread::spawn(move || {
            let mut buffer = [0u8; 64];
            loop {
                if let Ok(()) = rx_stop.try_recv() {
                    break;
                }
                {
                    let mut conn_guard = conn_reader.lock().unwrap();
                    match conn_guard.read(&mut buffer) {
                        Ok(size) if size > 0 => {
                            let s = String::from_utf8_lossy(&buffer[..size]);
                            print!("{}", s);
                            io::stdout().flush().unwrap();
                        }
                        _ => {}
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });

        let original_mode = set_raw_mode();
        eprintln!("Raw mode enabled. Type into the terminal to send data. Press 'q' to quit.");

        loop {
            let mut buffer = [0u8; 1]; // Read one byte at a time
            if io::stdin().read(&mut buffer).is_ok() {
                // Check for 'q' to quit
                if buffer[0] == b'q' {
                    println!("Exiting...");
                    break;
                }
        
                // Handle Enter key by sending `\r`
                if buffer[0] == b'\n' {
                    let mut conn_guard = active_conn.lock().unwrap();
                    conn_guard.write(b"\r").unwrap(); // That's what puttys default behaviour is -> Maybe change to b"\r\n in future" or make it configurable?
                } else {
                    // Send the input byte directly
                    let mut conn_guard = active_conn.lock().unwrap();
                    conn_guard.write(&buffer).unwrap();
                }
            }
        }

        let _ = tx_stop.send(());
        restore_mode(original_mode);
        manager.destroy_connection(Arc::try_unwrap(active_conn).unwrap().into_inner().unwrap())?;
    } else {
        eprintln!("No --port argument provided.");
        eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
        eprintln!("Or Usage: cargo run -- --port /dev/pts/3 --baud 115200")
    }

    Ok(())
}
