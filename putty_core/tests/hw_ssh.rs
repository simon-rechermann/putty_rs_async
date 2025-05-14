//! Integration test that spins‑up a throw‑away OpenSSH **server
//! in the foreground** and talks to it over 127.0.0.1 using a
//! throw‑away key‑pair.

#![cfg(feature = "hw-tests")]

use anyhow::{Context, Result};
// use openssh::{KnownHosts, SessionBuilder, Stdio};
use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    process::{Child, Command},
    thread::sleep,
    time::{Duration, Instant},
};
use tempfile::{tempdir, NamedTempFile};
use which::which;
use putty_core::{connections::ssh::ssh_connection::SshConnection, ConnectionManager};

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Grab any free high port by asking the OS for port 0
fn free_tcp_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Wait until `127.0.0.1:port` starts accepting connections
fn wait_until_listening(port: u16, timeout_ms: u64) {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(30));
    }
    panic!("sshd did not start on port {port}");
}

// ---------------------------------------------------------------------------
// The actual test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sshd_echo_roundtrip() -> Result<()> {
    // ── 1. workspace (auto‑deleted) + free port ────────────────────────────
    let workdir = tempdir()?;
    let port = free_tcp_port();

    // ── 2. generate **server** host key  (ssh‑host‑key) ────────────────────
    let host_key = workdir.path().join("host_ed25519");
    Command::new(which("ssh-keygen")?)
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&host_key)
        .status()
        .context("failed to create host key")?;

    // ── 3. generate **client** key pair  (used by this test) ───────────────
    let client_key = workdir.path().join("client_ed25519");
    Command::new(which("ssh-keygen")?)
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&client_key)
        .status()
        .context("failed to create client key")?;

    // authorise that client key for *any* local user running the test
    let authorized_keys = workdir.path().join("authorized_keys");
    fs::copy(client_key.with_extension("pub"), &authorized_keys)?;
    fs::set_permissions(&authorized_keys, fs::Permissions::from_mode(0o600))?;

    // ── 4. minimal sshd_config written to a temp file ──────────────────────
    let mut cfg = NamedTempFile::new_in(workdir.path())?;
    writeln!(
        cfg,
        r#"
Port {port}
ListenAddress 127.0.0.1
HostKey {host_key}

# auth: keys only, no PAM
PasswordAuthentication no
PubkeyAuthentication  yes
AuthorizedKeysFile    {authorized_keys}
UsePAM                no
StrictModes           no          # ← allow /tmp parent dir

PidFile {pidfile}
LogLevel QUIET                  # ← set to DEBUG3 for more info
"#,
        port = port,
        host_key = host_key.display(),
        authorized_keys = authorized_keys.display(),
        pidfile = workdir.path().join("sshd.pid").display(),
    )?;
    cfg.flush()?;

    // ── 5. start sshd in the foreground (-D) so we can kill it later ───────
    let sshd_path = which("sshd")?;
    let mut sshd: Child = Command::new(sshd_path)
        .args(["-e", "-D", "-f"])
        .arg(cfg.path())
        .spawn()
        .context("unable to launch sshd")?;

    wait_until_listening(port, 2_000);

    // ── 6. client side: connect with the key we just made ──────────────────
    let user: String = std::env::var("USER").expect("USER env var is needed for ssh test but not set");
 
    let conn = SshConnection::with_key(
        "127.0.0.1".into(),
        port,
        user.clone(),
        client_key.clone(),   // ← same key we just authored
        None,                 // passphrase
    );

    let manager = ConnectionManager::new();
    manager
        .add_connection("ssh".into(), Box::new(conn))
        .await
        .expect("add_connection failed");

    let mut rx = manager
        .subscribe("ssh")
        .await
        .expect("subscribe failed");

    // ── 7. round‑trip ----------------------------------------------------------
    manager.write_bytes("ssh", b"hi\n").await?;

    // Pull chunks until one of them contains the bytes h‑i (max 2 s)
    let echoed: Vec<u8> = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let chunk = rx.recv().await.expect("channel closed");  // Result → Vec<u8>
            if chunk.windows(2).any(|w| w == b"hi") {
                break chunk;                                       // success
            }
        }
    })
    .await?;   // propagate timeout = test failure

    // A real assertion: make sure the bytes are in the buffer we kept
    assert!(
        echoed.windows(2).any(|w| w == b"hi"),
        "did not find 'hi' in echoed data: {:?}",
        echoed
    );

    log::info!("received: {:?}", String::from_utf8_lossy(&echoed));

    // ── 8. tidy up (unchanged) ────────────────────────────────────────────────
    manager.stop_connection("ssh").await.ok();
    sshd.kill().ok();
    sshd.wait().ok();

    Ok(())
}
