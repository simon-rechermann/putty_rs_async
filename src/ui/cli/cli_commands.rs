use clap::{Parser, Subcommand};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use log::info;
use std::io::{self, Read};

use crate::connections::errors::ConnectionError;
use crate::connections::serial::SerialConnection;
use crate::connections::ssh::ssh_connection::SshConnection;
use crate::core::connection_manager::{ConnectionHandle, ConnectionManager};

/// Enable raw mode via crossterm, throwing an error if it fails.
/// This disables line-buffering and echo on all supported platforms.
fn set_raw_mode() -> Result<(), ConnectionError> {
    enable_raw_mode()
        .map_err(|e| ConnectionError::Other(format!("Failed to enable raw mode: {}", e)))
}

/// Restore normal terminal mode.
/// crossterm internally remembers the previous mode and restores it.
fn restore_mode() {
    let _ = disable_raw_mode();
}

/// Command-line arguments.
#[derive(Parser, Debug)]
#[command(name = "putty_rs", version = "0.1.0", subcommand_required = true)]
pub struct Args {
    /// Launch in GUI mode
    #[arg(long)]
    pub gui: bool,

    #[command(subcommand)]
    pub protocol: Protocol,
}

#[derive(Subcommand, Debug)]
pub enum Protocol {
    /// Use a serial connection
    Serial {
        /// Serial port to open (default on UNIX: `/dev/pts/3`)
        /// If you're on Windows, you might specify something like `COM3`.
        #[arg(long, default_value = "/dev/pts/3")]
        port: String,
        /// Baud rate (default 115200)
        #[arg(long, default_value_t = 115200)]
        baud: u32,
    },
    /// Use an SSH connection
    Ssh {
        /// SSH server host
        #[arg(long)]
        host: String,
        /// SSH server port (default 22)
        #[arg(long, default_value_t = 22)]
        port: u16,
        /// Username for SSH authentication
        #[arg(long)]
        username: String,
        /// Password for SSH authentication
        #[arg(long, default_value = "")]
        password: String,
    },
}

pub fn run_cli(args: Args) -> Result<(), ConnectionError> {
    // Create a ConnectionManager to manage connections.
    let connection_manager = ConnectionManager::new();

    // Callback for incoming bytes: simply print them to stdout.
    let on_byte = |byte: u8| {
        print!("{}", byte as char);
    };

    match args.protocol {
        Protocol::Serial { port, baud } => {
            run_serial_protocol(port, baud, &connection_manager, on_byte)?;
        },
        Protocol::Ssh { host, port, username, password } => {
            run_ssh_protocol(host, port, username, password, &connection_manager, on_byte)?;
        },
    }

    Ok(())
}

/// Run the CLI for the serial connection.
fn run_serial_protocol(
    port: String,
    baud: u32,
    connection_manager: &ConnectionManager,
    on_byte: impl Fn(u8) + Send + 'static,
) -> Result<(), ConnectionError> {
    info!("Opening serial port: {} at {} baud", port, baud);
    let conn = SerialConnection::new(port.clone(), baud);
    let handle: ConnectionHandle = connection_manager.add_connection(port.clone(), Box::new(conn), on_byte)?;

    info!("Enable raw mode. Press Ctrl+A then 'x' to exit the program.");
    set_raw_mode()?;
    let mut last_was_ctrl_a = false;
    let mut buf = [0u8; 1];
    while io::stdin().read(&mut buf).is_ok() {
        let ch = buf[0];

        // If user typed Ctrl+A (ASCII 0x01), set a flag
        if ch == 0x01 {
            last_was_ctrl_a = true;
            continue;
        }

        // If the previous character was Ctrl+A and the user typed 'x', exit
        // and restore terminal mode
        if last_was_ctrl_a && ch == b'x' {
            restore_mode();
            info!("Exiting...");
            break;
        } else {
            last_was_ctrl_a = false;
        }

        // Optionally convert carriage return to something else
        if ch == b'\r' {
            let _ = handle.write_bytes(b"\r");
        } else {
            let _ = handle.write_bytes(&[ch]);
        }
    }
    let _ = handle.stop();
    info!("Terminal mode restored.");
    Ok(())
}

/// Run the CLI for the SSH connection.
fn run_ssh_protocol(
    host: String,
    port: u16,
    username: String,
    password: String,
    connection_manager: &ConnectionManager,
    on_byte: impl Fn(u8) + Send + 'static,
) -> Result<(), ConnectionError> {
    info!("Connecting to SSH server {}:{} as user {}", host, port, username);
    let conn = SshConnection::new(host.clone(), port, username.clone(), password.clone());
    let handle: ConnectionHandle = connection_manager.add_connection(host.clone(), Box::new(conn), on_byte)?;

    info!("SSH session started. Press Ctrl+C to exit.");
    let mut buf = [0u8; 1];
    while io::stdin().read(&mut buf).is_ok() {
        let ch = buf[0];
        let _ = handle.write_bytes(&[ch]);
    }
    let _ = handle.stop();
    Ok(())
}
