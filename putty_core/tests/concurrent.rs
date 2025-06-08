use log::LevelFilter;
use putty_core::ConnectionManager;
use tokio::{
    sync::broadcast::Receiver,
    time::{timeout, Duration},
};

mod common;
use common::fake_connection::FakeConnection;

#[tokio::test]
async fn bytes_from_two_independent_connections_do_not_get_mixed() {
    //   Logs will appear only when you run with `-- --nocapture`
    //   or when the test fails.
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let connection_manager = ConnectionManager::new();

    // Build two independent fake connections
    let (fake_connection_port_a, to_port_a_tx, _from_port_a_rx) = FakeConnection::new();

    let (fake_connection_port_b, to_port_b_tx, _from_port_b_rx) = FakeConnection::new();

    // Register connections in the manager
    connection_manager
        .add_connection("PortA".into(), Box::new(fake_connection_port_a))
        .await
        .expect("adding PortA should succeed");

    connection_manager
        .add_connection("PortB".into(), Box::new(fake_connection_port_b))
        .await
        .expect("adding PortB should succeed");

    // Subscribe to each connection's broadcast stream
    let mut receiver_port_a: Receiver<Vec<u8>> = connection_manager
        .subscribe("PortA")
        .await
        .expect("PortA must exist");

    let mut receiver_port_b: Receiver<Vec<u8>> = connection_manager
        .subscribe("PortB")
        .await
        .expect("PortB must exist");

    // Inject a different byte into each fake connection
    // These bytes simulate data arriving from two *real* devices at (roughly)
    // the same moment.
    to_port_a_tx.send(b"a".to_vec()).await.unwrap();
    to_port_b_tx.send(b"b".to_vec()).await.unwrap();

    // Receive the bytes through the manager's broadcast channels
    // A small timeout converts hangs into readable test failures.
    let packet_from_port_a = timeout(Duration::from_millis(100), receiver_port_a.recv())
        .await
        .expect("timed out waiting for PortA")
        .expect("broadcast channel for PortA closed unexpectedly");

    let packet_from_port_b = timeout(Duration::from_millis(100), receiver_port_b.recv())
        .await
        .expect("timed out waiting for PortB")
        .expect("broadcast channel for PortB closed unexpectedly");

    // Assert that the streams never got crossed ────────────────────────
    assert_eq!(
        packet_from_port_a, b"a",
        "PortA should receive only its own bytes"
    );
    assert_eq!(
        packet_from_port_b, b"b",
        "PortB should receive only its own bytes"
    );
}
