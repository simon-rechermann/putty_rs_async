#![cfg(feature = "hw-tests")]

use openssh::{KnownHosts, SessionBuilder, Stdio};
use putty_core::{
    connections::ssh::ssh_connection::SshConnection,
    ConnectionManager,
};

use std::{env, net::TcpStream, time::Duration};
use tokio::time::timeout;
use log::LevelFilter;

/// Integration‑test helper: return `true` iff we can open `localhost:22`.
fn sshd_available() -> bool {
    TcpStream::connect_timeout(&("127.0.0.1:22".parse().unwrap()), Duration::from_millis(300))
        .is_ok()
}

#[tokio::test]
async fn local_ssh_echo_server() -> anyhow::Result<()> {
    // ── enable DEBUG logs unless caller overrides ───────────────────────────
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // ── Skip gracefully if there is no sshd running on port 22 ──────────────
    if !sshd_available() {
        eprintln!("skipping hw_ssh: no sshd on localhost:22");
        return Ok(());        // test counts as "pass" but is skipped
    }

    // ── 1. Connect to the local sshd using openssh's native mux impl ────────
    let ssh_session = SessionBuilder::default()
        .known_hosts_check(KnownHosts::Accept)
        .connect_mux("localhost")
        .await?;

    // Start a trivial echo loop on the remote side.
    ssh_session
        .command("sh")
        .arg("-c")
        .arg("while read l; do echo $l; done")
        .stdin(Stdio::piped())
        .spawn()
        .await?;

    // ── 2. putty_core side: open our own SshConnection to port 22 ───────────
    let username = env::var("USER").unwrap_or_else(|_| "nobody".into());

    let ssh_connection = SshConnection::new(
        "127.0.0.1".into(),
        22,
        username,
        "".into(),          // empty password triggers key‑based auth
    );

    let connection_manager = ConnectionManager::new();
    connection_manager
        .add_connection("ssh".into(), Box::new(ssh_connection))
        .await?;

    let mut broadcast_receiver = connection_manager
        .subscribe("ssh")
        .await
        .expect("subscribe failed");

    connection_manager
        .write_bytes("ssh", b"hi\n")
        .await?;

    let echoed = timeout(Duration::from_secs(2), broadcast_receiver.recv())
        .await?
        .expect("broadcast channel closed unexpectedly");

    assert_eq!(echoed, b"hi\n");
    Ok(())
}
