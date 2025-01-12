use clap::Parser;
use crate::core::connection::Connection;
use crate::core::{ConnectionManager, ConnectionError};
use crate::connections::serial::SerialConnection;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use termios::*;
use log::{info, error, debug};

fn set_raw_mode() -> Termios {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(stdin_fd).unwrap();

    let original_termios = termios.clone();

    // Disable canonical mode & echo, but DO NOT remove signals:
    termios.c_lflag &= !(ICANON | ECHO);

    // Minimum of 1 byte per read, no timeout
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

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
        let active_conn = Arc::new(Mutex::new(active_conn));

        // If you no longer want Ctrl+C to exit the program, you can remove or comment out this handler:
        /*
        ctrlc::set_handler(move || {
            // do nothing, or store a flag, but by default we won't exit on Ctrl+C
        }).expect("Error setting Ctrl+C handler");
        */

        // A simple atomic flag to let the reader thread know when to stop.
        let stop_flag = Arc::new(AtomicBool::new(false));

        // Spawn the reader thread
        let conn_reader = Arc::clone(&active_conn);
        let reader_stop_flag = Arc::clone(&stop_flag);
        let reader_thread = std::thread::spawn(move || {
            let mut buffer = [0u8; 1]; // Only read one byte at a time
            while !reader_stop_flag.load(Ordering::SeqCst) {
                {
                    let mut conn_guard = conn_reader.lock().unwrap();
                    match conn_guard.read(&mut buffer) {
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
                        Ok(0) => {
                            // No data (timeout or nothing to read)
                            debug!("Ok(0)");
                        }
                        Ok(_) => {
                            // This shouldn't happen for a 1-byte buffer, but we must match it
                            debug!("Ok(_)");
                        }
                        Err(ConnectionError::IoError(ref io_err))
                            if io_err.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            // Ignore timeouts, just check stop_flag again in the loop
                            debug!("Err(ConnErr) - TimedOut");
                        }
                        // Some other error
                        Err(_) => {
                            debug!("Err(_)");
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        let original_mode = set_raw_mode();
        eprintln!("Raw mode enabled. Type into the terminal to send data.");
        eprintln!("Press Ctrl+A then 'x' to exit.");

        // We'll track whether the last character was Ctrl+A.
        let mut last_was_ctrl_a = false;

        // Main loop for handling user input
        loop {
            let mut buffer = [0u8; 1];
            if io::stdin().read(&mut buffer).is_ok() {
                let ch = buffer[0];

                // 0x01 is ASCII for Ctrl+A
                if ch == 0x01 {
                    last_was_ctrl_a = true;
                    // Don't forward Ctrl+A to the serial port
                    continue;
                }

                if last_was_ctrl_a && ch == b'x' {
                    info!("Ctrl+A + x pressed. Exiting...");
                    break;
                } else {
                    // If the user pressed Ctrl+A but followed with anything other than 'x',
                    // we reset and treat `ch` normally.
                    last_was_ctrl_a = false;

                    if ch == b'\n' {
                        // Convert '\n' to '\r' when sending over serial
                        let mut conn_guard = active_conn.lock().unwrap();
                        conn_guard.write(b"\r")?;
                    } else {
                        // Send the input byte directly
                        let mut conn_guard = active_conn.lock().unwrap();
                        conn_guard.write(&[ch])?;
                    }
                }
            } else {
                // If there's an error reading from stdin, handle or break
                error!("Error reading from stdin (unexpected).");
                break;
            }
        }

        // Stop the reader thread
        stop_flag.store(true, Ordering::SeqCst);
        reader_thread.join().expect("Failed to join reader thread");

        // Restore the original terminal mode before exiting
        restore_mode(original_mode);
    } else {
        eprintln!("No --port argument provided.");
        eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
        eprintln!("Or usage: cargo run -- --port /dev/pts/3 --baud 115200");
    }

    Ok(())
}
