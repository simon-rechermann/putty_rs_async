use clap::Parser;
use crate::core::connection::Connection;
use crate::core::{ConnectionManager, ConnectionError};
use crate::connections::serial::SerialConnection;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use termios::*;
use log::{info, error, debug};


fn set_raw_mode() -> Termios {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(stdin_fd).unwrap();

    let original_termios = termios.clone();

    // Disable canonical mode & echo, but DO NOT remove signals:
    termios.c_lflag &= !(ICANON | ECHO);

    termios.c_cc[VMIN] = 1;  // Minimum number of characters for a read
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

        let stop_flag = Arc::new(AtomicBool::new(false));

        // Spawn reader thread
        let conn_reader = Arc::clone(&active_conn);
        let reader_stop_flag = Arc::clone(&stop_flag);

        let reader_thread = std::thread::spawn(move || {
            let mut buffer = [0u8; 1]; // Only read one byte at a time
            while !reader_stop_flag.load(Ordering::SeqCst) {
                {
                    let mut conn_guard = conn_reader.lock().unwrap();
                    let read_result = conn_guard.read(&mut buffer);
                    match read_result {
                        Ok(1) => {
                            let ch = buffer[0];
                            if ch == b'\r' {
                                print!("\r\n");
                            } else {
                                print!("{}", ch as char);
                            }
                            io::stdout().flush().unwrap();
                            debug!("Ok(1)");
                        }
                        // No data just yields Ok(0), just ignore
                        Ok(0) => {debug!("Ok(0)");}
                        Ok(_) => {
                            // anything else (2..=buffer.len())
                            // in practice if buffer = [0u8; 1], it won’t happen, but the compiler requires it
                            debug!("Ok(_)");
                        }
                        Err(ConnectionError::IoError(ref io_err)) if io_err.kind() == std::io::ErrorKind::TimedOut => {
                            // Ignore timeouts, just check stop_flag again in the loop
                            debug!("Err(ConnErr)");
                        }
                        // Some other error
                        Err(_) => {debug!("Err(_)");}
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10)); // Optional small delay
            }
        });

        // Handle Ctrl+C to trigger cleanup
        {
            let stop_flag = Arc::clone(&stop_flag);
            ctrlc::set_handler(move || {
                stop_flag.store(true, Ordering::SeqCst);
            }).expect("Error setting Ctrl+C handler");
        }

        let original_mode = set_raw_mode();
        eprintln!("Raw mode enabled. Type into the terminal to send data. Press 'q' to quit.");

        loop {
            let mut buffer = [0u8; 1]; // Read one byte at a time
            // Because we left ISIG enabled, Ctrl+C can still kill or interrupt this read,
            // triggering the ctrlc handler. Also we do not wait forever because the OS
            // will deliver SIGINT as soon as user hits Ctrl+C.
            if io::stdin().read(&mut buffer).is_ok() {
                // Check for 'q' to quit
                if buffer[0] == b'q' {
                    info!("Exiting...");
                    break;
                }
        
                // Handle Enter key by sending `\r` like putty does by default
                if buffer[0] == b'\n' {
                    let mut conn_guard = active_conn.lock().unwrap();
                    conn_guard.write(b"\r").unwrap();
                } else {
                    // Send the input byte directly
                    let mut conn_guard = active_conn.lock().unwrap();
                    conn_guard.write(&buffer).unwrap();
                }
            } else {
                error!("We should not end up here");
                // If there's an error reading from stdin, might also want to handle it or break
            }
        }

        // Signal reader thread to stop and wait for it
        stop_flag.store(true, Ordering::SeqCst);
        reader_thread.join().expect("Failed to join reader thread");

        // Restore the original terminal mode before exiting
        restore_mode(original_mode);
    } else {
        eprintln!("No --port argument provided.");
        eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
        eprintln!("Or Usage: cargo run -- --port /dev/pts/3 --baud 115200")
    }

    Ok(())
}
