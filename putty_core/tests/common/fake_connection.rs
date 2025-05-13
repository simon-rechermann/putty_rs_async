//! A deterministic **in‑process stand‑in** for any type that implements
//! `putty_core::connections::connection::Connection`.
//!
//! *  **From the test’s perspective**
//!    * Push “incoming” data into the connection with  
//!      `test_to_fake_tx.send(bytes).await`.
//!    * Inspect everything the manager wrote out via `fake_connection.write_history`.
//!
//! *  **Why this exists**: It lets integration tests exercise the *real* async
//!    machinery (tasks, channels, broadcasts) without opening a TCP socket or
//!    serial port.

use async_trait::async_trait;
use putty_core::connections::{connection::Connection, errors::ConnectionError};
use tokio::sync::mpsc;

pub struct FakeConnection {
    /// Bytes *pushed by the test* → appear as data read from the device.
    test_to_fake_rx: mpsc::Receiver<Vec<u8>>,
    /// Bytes written by the manager → sent back to the test
    fake_to_test_tx: mpsc::Sender<Vec<u8>>,

    /// Every chunk the manager wrote, kept for assertions.
    pub write_history: Vec<Vec<u8>>,
    pub connected: bool,
    pub disconnected: bool,
}

impl FakeConnection {
    /// Create a new fake plus two helper channels.
    ///
    /// Returns a triple:
    /// 1. `FakeConnection` – move this into `Box::new(...)` and hand it to
    ///    `ConnectionManager::add_connection`.
    /// 2. `test_to_fake_tx` – send bytes **into** the fake (simulated device input).
    /// 3. `fake_to_test_rx` – receive bytes the manager tried to write (optional).
    pub fn new(
    ) -> (
        Self,
        mpsc::Sender<Vec<u8>>,
        mpsc::Receiver<Vec<u8>>,
    ) {
        let (test_to_fake_tx, test_to_fake_rx) = mpsc::channel(32);
        let (fake_to_test_tx, fake_to_test_rx) = mpsc::channel(32);

        (
            Self {
                test_to_fake_rx,
                fake_to_test_tx,
                write_history: Vec::new(),
                connected: false,
                disconnected: false,
            },
            test_to_fake_tx,
            fake_to_test_rx,
        )
    }
}

#[async_trait]
impl Connection for FakeConnection {
    async fn connect(&mut self) -> Result<(), ConnectionError> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        self.disconnected = true;
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        // Record for later assertions …
        self.write_history.push(data.to_vec());

        // … and echo back through the helper channel (rarely used).
        let _ = self.fake_to_test_tx.send(data.to_vec()).await;
        Ok(data.len())
    }

    async fn read(&mut self, destination_buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        match self.test_to_fake_rx.recv().await {
            Some(mut incoming_chunk) => {
                // Copy as much as fits into the caller‑provided buffer.
                let bytes_to_copy = incoming_chunk.len().min(destination_buffer.len());
                destination_buffer[..bytes_to_copy]
                    .copy_from_slice(&incoming_chunk[..bytes_to_copy]);

                // If the chunk is larger than the buffer, push the remainder
                // back to the front of our internal queue so the next `read`
                // call can pick it up.  This mimics a real stream’s partial read.
                if incoming_chunk.len() > bytes_to_copy {
                    let remainder = incoming_chunk.split_off(bytes_to_copy);
                    let _ = self.fake_to_test_tx.send(remainder).await;
                }

                Ok(bytes_to_copy)
            }
            None => Err(ConnectionError::Other(
                "test cancelled the channel; no more data".into(),
            )),
        }
    }
}
