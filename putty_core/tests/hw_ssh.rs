#![cfg(feature = "hw-tests")]

use openssh::{KnownHosts, SessionBuilder, Stdio};
use putty_core::{
    connections::ssh::ssh_connection::SshConnection,
    ConnectionManager,
};

use tokio::time::{timeout, Duration};
use log::LevelFilter;

use tempfile::TempDir;      // <‑ tiny helper; add `tempfile = "3"` to dev‑deps
use std::env;

/// Round‑trip a single line through a disposable local sshd.
///
/// Requires the `native-mux` feature on the **openssh** crate and the `ssh`
/// binaries present in `PATH` (installed by default on most Unix boxes).
#[tokio::test]
async fn local_ssh_echo_server() -> anyhow::Result<()> {
    // ── Logger ───────────────────────────────────────────────────────────────
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // ── 1. Create a throw‑away directory for sshd state ─────────────────────
    let sshd_workspace: TempDir = tempfile::tempdir()?;

    // ── 2. Launch sshd + multiplex master automatically via openssh crate ──
    //
    // new_process_mux() starts sshd on a random free port and returns a
    // connected MuxSession whose .port() we can query.
    let ssh_session = SessionBuilder::new_process_mux(sshd_workspace)
        .known_hosts_check(KnownHosts::Accept)
        .user(env::var("USER").unwrap_or_else(|_| "nobody".into()))
        .connect()
        .await?;

    // Simple echo shell inside the session
    ssh_session
        .command("sh")
        .arg("-c")
        .arg("while read l; do echo $l; done")
        .stdin(Stdio::piped())
        .spawn()
        .await?;

    let chosen_port = ssh_session.port(); // now always available

    // ── 3. putty‑core side: connect to that sshd instance ───────────────────
    let ssh_connection = SshConnection::new(
        "127.0.0.1".into(),
        chosen_port,
        env::var("USER").unwrap_or_else(|_| "nobody".into()),
        "".into(), // libssh2 will use public‑key auth from $SSH_AUTH_SOCK
    );

    let connection_manager = ConnectionManager::new();
    connection_manager
        .add_connection("ssh".into(), Box::new(ssh_connection))
        .await
        .expect("add_connection failed");

    let mut broadcast_receiver = connection_manager
        .subscribe("ssh")
        .await
        .expect("subscribe failed");

    // ── 4. Round‑trip: write "hi\n" and expect "hi\n" back within 2 s ───────
    connection_manager
        .write_bytes("ssh", b"hi\n")
        .await
        .expect("write_bytes failed");

    let echoed = timeout(Duration::from_secs(2), broadcast_receiver.recv())
        .await
        .expect("timeout waiting for echo")?
        ;

    assert_eq!(echoed, b"hi\n");
    Ok(())
}
