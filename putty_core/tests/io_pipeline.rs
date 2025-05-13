use log::LevelFilter;
use putty_core::ConnectionManager;
use tokio::{
    sync::broadcast,
    time::{timeout, Duration},
};

mod common;
use common::fake_connection::FakeConnection;

#[tokio::test]
async fn roundtrip_and_write_path() {
    //   Logs will appear only when you run with `-- --nocapture`
    //   or when the test fails.
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // ── Setup ────────────────────────────────────────────────────────────
    let connection_manager = ConnectionManager::new();
    let (fake_connection, test_to_fake_tx, _fake_to_test_rx) = FakeConnection::new();

    connection_manager
        .add_connection("fakePort".into(), Box::new(fake_connection))
        .await
        .expect("add_connection should succeed");

    let mut subscriber_rx: broadcast::Receiver<Vec<u8>> = connection_manager
        .subscribe("fakePort")
        .await
        .expect("subscribe should succeed");

    // ── Round‑trip path (device → manager → subscriber) ──────────────────
    let incoming_bytes = b"hello\n".to_vec();
    test_to_fake_tx
        .send(incoming_bytes.clone())
        .await
        .expect("send into fake should succeed");

    let echoed_bytes = timeout(Duration::from_millis(200), subscriber_rx.recv())
        .await
        .expect("timeout waiting for echo")
        .expect("broadcast channel closed unexpectedly");

    assert_eq!(
        echoed_bytes, incoming_bytes,
        "subscriber should receive the exact bytes injected into the fake connection"
    );

    // ── Write path (subscriber → manager → device) ───────────────────────
    let bytes_written = connection_manager
        .write_bytes("fakePort", b"AT\r")
        .await
        .expect("write_bytes should succeed");

    assert_eq!(
        bytes_written, 3,
        "write_bytes should report the number of bytes handed to the connection"
    );
}
