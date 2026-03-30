use clap::{Parser, Subcommand};
#[cfg(any(feature = "serial", feature = "ssh"))]
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
#[cfg(any(feature = "serial", feature = "ssh"))]
use log::info;
use putty_core::connections::errors::ConnectionError;
#[cfg(feature = "serial")]
use putty_core::connections::serial::SerialConnection;
#[cfg(feature = "ssh")]
use putty_core::connections::ssh::SshConnection;
#[cfg(any(feature = "serial", feature = "ssh"))]
use putty_core::connections::Connection;
#[cfg(any(feature = "serial", feature = "ssh"))]
use putty_core::core::connection_manager::ConnectionManager;
use putty_core::{Profile, ProfileStore};
#[cfg(any(feature = "serial", feature = "ssh"))]
use std::io::{stdout, Write};
#[cfg(any(feature = "serial", feature = "ssh"))]
use tokio::io::{self, AsyncReadExt};

/// Enable raw mode via crossterm, throwing an error if it fails.
/// This disables line-buffering and echo on all supported platforms.
#[cfg(any(feature = "serial", feature = "ssh"))]
fn set_raw_mode() -> Result<(), ConnectionError> {
    enable_raw_mode().map_err(|e| ConnectionError::Other(format!("Failed to enable raw mode: {e}")))
}

/// Restore normal terminal mode.
/// crossterm internally remembers the previous mode and restores it.
#[cfg(any(feature = "serial", feature = "ssh"))]
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
    #[cfg(feature = "serial")]
    /// Use a serial connection
    Serial {
        /// Serial port to open
        #[arg(long, default_value = "/dev/pts/3")]
        port: String,
        /// Baud rate
        #[arg(long, default_value_t = 115200)]
        baud: u32,
    },
    #[cfg(feature = "ssh")]
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
    /// Manage saved connection presets.
    Storage {
        #[command(subcommand)]
        action: StorageAction,
    },
}

/// Actions in `putty_rs storage <action>`
#[derive(Subcommand, Debug)]
pub enum StorageAction {
    List,
    #[cfg(feature = "serial")]
    SaveSerial {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "/dev/pts/3")]
        port: String,
        #[arg(long, default_value_t = 115200)]
        baud: u32,
    },
    #[cfg(feature = "ssh")]
    SaveSsh {
        #[arg(long)]
        name: String,
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
        #[arg(long)]
        username: String,
        #[arg(long, default_value = "")]
        password: String,
    },
    Delete {
        #[arg(long)]
        name: String,
    },
    UseProfile {
        #[arg(long)]
        profile: String,
    },
}

pub async fn run_cli(args: Args) -> Result<(), ConnectionError> {
    #[cfg(any(feature = "serial", feature = "ssh"))]
    let connection_manager = ConnectionManager::new();

    match args.protocol {
        #[cfg(feature = "serial")]
        Protocol::Serial { port, baud } => {
            run_serial_protocol(port, baud, &connection_manager).await?;
        }
        #[cfg(feature = "ssh")]
        Protocol::Ssh {
            host,
            port,
            username,
            password,
        } => {
            run_ssh_protocol(host, port, username, password, &connection_manager).await?;
        }
        Protocol::Storage { action } => match action {
            // open by profile name
            StorageAction::UseProfile { profile } => {
                let store =
                    ProfileStore::new().map_err(|e| ConnectionError::Other(e.to_string()))?;
                let preset = store
                    .list()?
                    .into_iter()
                    .find(|p| p.name() == profile)
                    .ok_or_else(|| {
                        ConnectionError::Other(format!("preset not found: {profile}"))
                    })?;

                match preset {
                    #[cfg(feature = "serial")]
                    Profile::Serial { port, baud, .. } => {
                        run_serial_protocol(port, baud, &connection_manager).await?
                    }
                    #[cfg(not(feature = "serial"))]
                    Profile::Serial { .. } => {
                        return Err(ConnectionError::Other(
                            "This CLI was built without serial support".into(),
                        ));
                    }
                    #[cfg(feature = "ssh")]
                    Profile::Ssh {
                        host,
                        port,
                        username,
                        password,
                        ..
                    } => {
                        run_ssh_protocol(host, port, username, password, &connection_manager)
                            .await?
                    }
                    #[cfg(not(feature = "ssh"))]
                    Profile::Ssh { .. } => {
                        return Err(ConnectionError::Other(
                            "This CLI was built without SSH support".into(),
                        ));
                    }
                }
            }

            StorageAction::List => {
                handle_storage_cmd(action).await?;
            }
            #[cfg(feature = "serial")]
            StorageAction::SaveSerial { .. } => {
                handle_storage_cmd(action).await?;
            }
            #[cfg(feature = "ssh")]
            StorageAction::SaveSsh { .. } => {
                handle_storage_cmd(action).await?;
            }
            StorageAction::Delete { .. } => {
                handle_storage_cmd(action).await?;
            }
        },
    }
    Ok(())
}

#[cfg(feature = "serial")]
async fn run_serial_protocol(
    port: String,
    baud: u32,
    connection_manager: &ConnectionManager,
) -> Result<(), ConnectionError> {
    info!("Opening serial port: {port} at {baud} baud");
    let conn = SerialConnection::new(port.clone(), baud);
    run_cli_loop(connection_manager, port, Box::new(conn)).await
}

#[cfg(feature = "ssh")]
async fn run_ssh_protocol(
    host: String,
    port: u16,
    username: String,
    password: String,
    connection_manager: &ConnectionManager,
) -> Result<(), ConnectionError> {
    info!("Connecting to SSH server {host}:{port} as user {username}");
    let conn = SshConnection::new(host.clone(), port, username, password);
    run_cli_loop(connection_manager, host, Box::new(conn)).await
}

/// Runs the CLI loop for a given connection.
///
/// This function registers a connection by passing ownership of the Connection trait object
/// (via `Box<dyn Connection + Send + Unpin>`)
/// to the connection manager, enables raw terminal mode, and reads user input to write to the connection.
/// It exits when the user types Ctrl+A followed by 'x',
#[cfg(any(feature = "serial", feature = "ssh"))]
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

async fn handle_storage_cmd(action: StorageAction) -> Result<(), ConnectionError> {
    let store = ProfileStore::new().map_err(|e| ConnectionError::Other(e.to_string()))?;

    match action {
        StorageAction::List => {
            for p in store.list()? {
                println!("{p:?}");
            }
        }
        #[cfg(feature = "serial")]
        StorageAction::SaveSerial { name, port, baud } => {
            store.save(&Profile::Serial { name, port, baud })?;
        }
        #[cfg(feature = "ssh")]
        StorageAction::SaveSsh {
            name,
            host,
            port,
            username,
            password,
        } => {
            store.save(&Profile::Ssh {
                name,
                host,
                port,
                username,
                password,
                keyring_id: None, // not needed here
            })?;
        }
        StorageAction::Delete { name } => {
            if !store.delete(&name)? {
                eprintln!("No such profile: {name}");
            }
        }
        StorageAction::UseProfile { .. } => unreachable!(), // handled above
    }
    Ok(())
}
