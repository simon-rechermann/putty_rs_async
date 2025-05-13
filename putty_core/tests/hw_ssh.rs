// tests/hw_ssh.rs
#![cfg(feature = "hw-tests")]

use openssh::{Session, KnownHosts, Stdio};
use putty_core::{connections::ssh::ssh_connection::SshConnection, ConnectionManager};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn local_ssh_echo_server() -> anyhow::Result<()> {
    //   Logs will appear only when you run with `-- --nocapture`
    //   or when the test fails.
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)  
        .is_test(true)
        .try_init();

    // spins up a disposable sshd in a temp dir
    let session = Session::new().known_hosts_check(KnownHosts::Accept).connect_mux("localhost").await?;
    session.command("sh")
        .arg("-c")
        .arg("while read l; do echo $l; done")
        .stdin(Stdio::piped())
        .spawn()
        .await?;
    let port = session.port(); // the forwarded port

    // Putty‑core side
    let connection = SshConnection::new("127.0.0.1".into(), port, "nobody".into(), "".into());
    let manager = ConnectionManager::new();
    manager.add_connection("ssh".into(), Box::new(connection)).await.unwrap();
    let mut rx = manager.subscribe("ssh").await.unwrap();

    manager.write_bytes("ssh", b"hi\n").await.unwrap();
    let echoed = timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    assert_eq!(echoed, b"hi\n");
    Ok(())
}
