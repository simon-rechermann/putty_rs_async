use clap::Parser;
use log::info;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;

use termios::*;
use crate::core::ConnectionManager;
use crate::connections::errors::ConnectionError;
use crate::connections::serial::SerialConnection;
use crate::core::session::Session;

/// Put stdin into raw mode so we can read each keystroke immediately.
fn set_raw_mode() -> Result<Termios, ConnectionError> {
    let stdin_fd = io::stdin().as_raw_fd();
    let mut termios = Termios::from_fd(stdin_fd)?;
    let original = termios.clone();

    // Disable canonical mode & echo
    termios.c_lflag &= !(ICANON | ECHO);

    // 1 byte at a time, no timeout
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;

    tcsetattr(stdin_fd, TCSANOW, &termios)?;
    Ok(original)
}

fn restore_mode(original: Termios) {
    let stdin_fd = io::stdin().as_raw_fd();
    let _ = tcsetattr(stdin_fd, TCSANOW, &original);
}

/// Command-line arguments struct.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0")]
pub struct Args {
    /// Launch in GUI mode
    #[arg(long)]
    pub gui: bool,
    #[arg(long, default_value = "/dev/pts/3")]
    pub port: Option<String>,
    #[arg(long, default_value_t = 115200)]
    pub baud: u32,
}

pub fn run_cli(args: Args) -> Result<(), ConnectionError> {

    if let Some(port) = args.port {
        eprintln!("Opening serial port: {} at {} baud", port, args.baud);

        // Prepare manager and connection
        let manager = ConnectionManager::new();
        let conn = SerialConnection::new(port.clone(), args.baud);
        let conn = manager.create_connection(conn)?;

        // The callback for each incoming byte: print it immediately
        let on_byte = move |byte: u8| {
            if byte == b'\r' {
                // If you want \r to appear as a newline:
                print!("\r\n");
            } else {
                print!("{}", byte as char);
            }
            let _ = io::stdout().flush();
        };

        // Build our session
        let mut session = Session::new(manager, Box::new(conn), on_byte);
        session.start();

        // Put terminal in raw mode so we get each typed character
        let original_mode = set_raw_mode()?;
        eprintln!("Raw mode enabled. Type into the terminal to send data. Press Ctrl+A then 'x' to exit.");

        let mut last_was_ctrl_a = false;
        let mut buf = [0u8; 1];

        // Main loop reading from stdin, sending each char immediately
        while io::stdin().read(&mut buf).is_ok() {
            let ch = buf[0];

            // Check for Ctrl+A then 'x' to quit
            if ch == 0x01 {
                last_was_ctrl_a = true;
                continue;
            }
            if last_was_ctrl_a && ch == b'x' {
                info!("Ctrl+A + x pressed. Exiting...");
                break;
            } else {
                last_was_ctrl_a = false;
            }

            // Convert newline to carriage return if desired
            if ch == b'\n' {
                let _ = session.write_bytes(b"\r");
            } else {
                let _ = session.write_bytes(&[ch]);
            }
        }

        // Cleanup
        session.stop().ok();
        restore_mode(original_mode);
        eprintln!("Terminal mode restored.");

    } else {
        eprintln!("No --port argument provided.");
        eprintln!("Usage: putty_rs --port /dev/ttyUSB0 --baud 115200");
        eprintln!("Or usage: cargo run -- --port /dev/pts/3 --baud 115200");
    }

    Ok(())
}
