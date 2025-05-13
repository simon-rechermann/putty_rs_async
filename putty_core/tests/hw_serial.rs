// tests/hw_serial.rs
#![cfg(feature = "hw-tests")]

use putty_core::{connections::serial::serial_connection::SerialConnection, ConnectionManager};
use tokio::time::{timeout, Duration};
use std::env;

#[tokio::test]
async fn virtual_serial_roundtrip() {
    // Requires a virtual pair created with tty0tty (Linux) or com0com (Windows).
    let left = env::var("VIRTUAL_SERIAL_LEFT").expect("set VIRTUAL_SERIAL_LEFT=/dev/tnt0");
    let right = env::var("VIRTUAL_SERIAL_RIGHT").expect("set VIRTUAL_SERIAL_RIGHT=/dev/tnt1");

    // open the “right” side in a helper task and echo everything we read
    tokio::spawn(async move {
        use tokio::{io::{AsyncReadExt, AsyncWriteExt}, fs::OpenOptions};
        let mut port = tokio_serial::new(right, 115_200).open_native_async().unwrap();
        let mut buf = [0u8; 64];
        loop {
            let n = port.read(&mut buf).await.unwrap();
            if n == 0 { continue }
            port.write_all(&buf[..n]).await.unwrap();
        }
    });

    let conn = SerialConnection::new(left.clone(), 115_200);
    let mgr = ConnectionManager::new();
    mgr.add_connection("dev".into(), Box::new(conn)).await.unwrap();
    let mut rx = mgr.subscribe("dev").await.unwrap();

    mgr.write_bytes("dev", b"ping").await.unwrap();
    let echo = timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
    assert_eq!(echo, b"ping");
}
