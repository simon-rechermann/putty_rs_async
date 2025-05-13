use clap::{Parser, Subcommand};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use log::info;
use putty_core::connections::errors::ConnectionError;
use putty_core::connections::serial::SerialConnection;
use putty_core::connections::ssh::SshConnection;
use putty_core::connections::Connection;
use putty_core::core::connection_manager::ConnectionManager;
use std::io::{stdout, Write};
use tokio::io::{self, AsyncReadExt};

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
    #[command(subcommand)]
    pub protocol: Protocol,
}

#[derive(Subcommand, Debug)]
pub enum Protocol {
    /// Use a serial connection
    Serial {
        /// Serial port to open
        #[arg(long, default_value = "/dev/pts/3")]
        port: String,
        /// Baud rate
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

pub async fn run_cli(args: Args) -> Result<(), ConnectionError> {
    let connection_manager = ConnectionManager::new();

    match args.protocol {
        Protocol::Serial { port, baud } => {
            run_serial_protocol(port, baud, &connection_manager).await?;
        }
        Protocol::Ssh {
            host,
            port,
            username,
            password,
        } => {
            run_ssh_protocol(host, port, username, password, &connection_manager).await?;
        }
    }
    Ok(())
}

async fn run_serial_protocol(
    port: String,
    baud: u32,
    connection_manager: &ConnectionManager,
) -> Result<(), ConnectionError> {
    info!("Opening serial port: {} at {} baud", port, baud);
    let conn = SerialConnection::new(port.clone(), baud);
    run_cli_loop(connection_manager, port, Box::new(conn)).await
}

async fn run_ssh_protocol(
    host: String,
    port: u16,
    username: String,
    password: String,
    connection_manager: &ConnectionManager,
) -> Result<(), ConnectionError> {
    info!(
        "Connecting to SSH server {}:{} as user {}",
        host, port, username
    );
    let conn = SshConnection::new(host.clone(), port, username, password);
    run_cli_loop(connection_manager, host, Box::new(conn)).await
}

/// Runs the CLI loop for a given connection.
///
/// This function registers a connection by passing ownership of the Connection trait object
/// (via `Box<dyn Connection + Send + Unpin>`)
/// to the connection manager, enables raw terminal mode, and reads user input to write to the connection.
/// It exits when the user types Ctrl+A followed by 'x',
async fn run_cli_loop(
    connection_manager: &ConnectionManager,
    id: String,
    conn: Box<dyn Connection + Send + Unpin>,
) -> Result<(), ConnectionError> {

    connection_manager.add_connection(id.clone(), conn).await?;

    // Subscribe to messages from the new connection
    let mut connection_receiver = connection_manager.subscribe(&id).await.unwrap();

    // -> echo to the user’s terminal
    tokio::spawn(async move {
        while let Ok(chunk) = connection_receiver.recv().await {
            let _ = stdout().write_all(&chunk);
            let _ = stdout().flush();
        }
    });

    info!("Enable raw mode. Press Ctrl+A then 'x' to exit the program.");
    set_raw_mode()?;

    let mut last_was_ctrl_a = false;
    let mut buf = [0u8; 1];
    let mut stdin = io::stdin();
    loop {
        if stdin.read_exact(&mut buf).await.is_err() {
            break;
        }
        let ch = buf[0];
        if ch == 0x01 {
            last_was_ctrl_a = true;
            continue;
        }
        if last_was_ctrl_a && ch == b'x' {
            restore_mode();
            info!("Exiting...");
            break;
        } else {
            last_was_ctrl_a = false;
        }
        if ch == b'\r' {
            let _ = connection_manager.write_bytes(&id, b"\r").await;
        } else {
            let _ = connection_manager.write_bytes(&id, &[ch]).await;
        }
    }
    let _ = connection_manager.stop_connection(&id).await;
    info!("Terminal mode restored.");
    Ok(())
}
