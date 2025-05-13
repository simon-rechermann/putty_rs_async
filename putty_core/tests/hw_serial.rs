//! hw_serial.rs – round‑trip test through a virtual serial pair created
//! on‑the‑fly with `socat`.  Built **only** on Unix when the `hw-tests`
//! feature is enabled.
#![cfg(feature = "hw-tests")]
#![cfg(unix)]

use putty_core::{connections::serial::serial_connection::SerialConnection, ConnectionManager};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::{timeout, Duration},
};

use log::LevelFilter;
use std::path::PathBuf;
use tokio_serial::SerialPortBuilderExt;

/// Spawn `socat -d -d pty,raw,echo=0 pty,raw,echo=0` and capture the two PTY
/// device paths it prints.  Returns `(left, right, child_handle)`.
async fn spawn_socat_pair() -> anyhow::Result<(PathBuf, PathBuf, Child)> {
    use regex::Regex;

    // 1. start socat
    let mut socat_child = Command::new("socat")
        .arg("-d")
        .arg("-d")
        .arg("pty,raw,echo=0")
        .arg("pty,raw,echo=0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped()) // socat writes its messages here
        .spawn()?;

    // 2. read its stderr for two "opening character device" lines
    let mut stderr_lines =
        BufReader::new(socat_child.stderr.take().expect("capturing socat stderr")).lines();

    let virtual_device_regex = Regex::new(r#"(/dev/[^\s"]+)"#)?;
    let mut pty_paths: Vec<PathBuf> = Vec::with_capacity(2);

    while let Some(line) = stderr_lines.next_line().await? {
        log::debug!("socat: {}", line);
        if let Some(caps) = virtual_device_regex.captures(&line) {
            pty_paths.push(PathBuf::from(&caps[1]));
            if pty_paths.len() == 2 {
                break;
            }
        }
    }

    if pty_paths.len() != 2 {
        return Err(anyhow::anyhow!("socat did not report two PTYs"));
    }

    Ok((pty_paths[0].clone(), pty_paths[1].clone(), socat_child))
}

#[tokio::test]
async fn virtual_serial_roundtrip() {
    // ── Logger: DEBUG by default, but RUST_LOG can override ───────────────────
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // ── Obtain a fresh virtual port pair via socat ────────────────────────────
    let (left_pty_path, right_pty_path, _socat_child) =
        spawn_socat_pair().await.expect("failed to spawn socat");

    log::info!(
        "Using virtual ports: LEFT = {:?}, RIGHT = {:?}",
        left_pty_path,
        right_pty_path
    );

    // ── Helper task: echo back everything read on the RIGHT port ──────────────
    tokio::spawn({
        let right_path_string = right_pty_path.to_string_lossy().into_owned();
        async move {
            let mut echo_port = tokio_serial::new(right_path_string, 115_200)
                .open_native_async()
                .expect("failed to open right PTY");

            let mut buffer = [0u8; 64];
            loop {
                let bytes_read = echo_port.read(&mut buffer).await.unwrap();
                if bytes_read > 0 {
                    echo_port.write_all(&buffer[..bytes_read]).await.unwrap();
                }
            }
        }
    });

    // ── Code under test: open the LEFT port via SerialConnection ──────────────
    let left_path_string = left_pty_path.to_string_lossy().into_owned();
    let serial_connection = SerialConnection::new(left_path_string, 115_200);

    let connection_manager = ConnectionManager::new();
    connection_manager
        .add_connection("dev".into(), Box::new(serial_connection))
        .await
        .expect("add_connection failed");

    let mut broadcast_receiver = connection_manager
        .subscribe("dev")
        .await
        .expect("subscribe failed");

    // ── Send "ping" and expect "ping" back within 1 s ─────────────────────────
    connection_manager
        .write_bytes("dev", b"ping")
        .await
        .expect("write_bytes failed");

    let echoed_frame = timeout(Duration::from_secs(1), broadcast_receiver.recv())
        .await
        .expect("timeout waiting for echo")
        .expect("broadcast channel closed unexpectedly");

    assert_eq!(echoed_frame, b"ping");
}
