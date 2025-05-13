use log::LevelFilter;
use putty_core::ConnectionManager;

mod common;
use common::fake_connection::FakeConnection;

#[tokio::test]
async fn stop_connection_removes_handle_and_second_call_errors() {
    //   Logs will appear only when you run with `-- --nocapture`
    //   or when the test fails.
    let _ = env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)  
        .is_test(true)
        .try_init();

    let connection_manager = ConnectionManager::new();
    let (fake_connection, ..) = FakeConnection::new();
    let connection_id = "test_id";

    connection_manager
        .add_connection(connection_id.into(), Box::new(fake_connection))
        .await
        .expect("adding the connection should succeed");

    // ── Act ─ first stop should succeed and clean up the hash‑map entry ──────
    connection_manager
        .stop_connection(connection_id)
        .await
        .expect("first stop should succeed");

    // ── Assert ─ a second stop must fail because the entry is gone ────────────
    let second_stop_error = connection_manager
        .stop_connection(connection_id)
        .await
        .expect_err("second stop should fail; the connection is already removed");

    assert!(
        second_stop_error.to_string().contains("No connection"),
        "error message should mention that the connection no longer exists"
    );
}