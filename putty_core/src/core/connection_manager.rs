use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;
use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};

enum IoEvent {
    Write(Vec<u8>),
    Stop,
}
/// Represents the I/O task handle for a connection.
///
/// 1. ConnectionIOHandle holds the IO task that reads from the connection
/// and broadcasts the read messages(via broadcast_tx) to all its listeners (e.g. the cli).
/// 2. It exposes write_stop_tx to the public API (write_bytes, stop_connection)
/// allowing UIs to send messages to the connection.
struct ConnectionIOHandle {
    io_task_handle: tokio::task::JoinHandle<()>,
    write_stop_tx: mpsc::Sender<IoEvent>,
    broadcast_tx: broadcast::Sender<Vec<u8>>,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    manager: ConnectionManager,
    id: String,
}

/// Manages multiple connections concurrently.
///
/// The internal state is a HashMap that maps unique connection identifiers to their
/// corresponding ConnectionIOHandle. The use of an Arc and a Mutex ensures that the
/// ConnectionManager can be safely shared across threads and cloned cheaply. Cloning
/// the ConnectionManager merely increases the reference count, so no deep copy of the
/// underlying data is performed. This allows for efficient sharing of the connection manager
/// via the ConnectionHandle.
#[derive(Clone)]
pub struct ConnectionManager {
    inner: Arc<Mutex<HashMap<String, ConnectionIOHandle>>>,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Adds a new connection to the ConnectionManager.
    /// - `id`: A unique identifier (e.g. port name or host)
    /// - `conn`: A *not-yet-connected* Connection
    ///   This method takes ownership of the connection, connects it,
    ///   and spawns an async I/O task.
    ///
    ///   It then returns a `ConnectionHandle` that can be used to control
    ///   the connection.
    pub async fn add_connection(
        &self,
        id: String,
        mut conn: Box<dyn Connection + Send + Unpin>,
    ) -> Result<ConnectionHandle, ConnectionError> {
        conn.connect().await?;

        // Broadcast messages from the connection to all listeners(UIs)
        // Listeners(having subscribes via public API) <- I/O task
        let (broadcast_tx, _) = broadcast::channel::<Vec<u8>>(256);

        // Channel public API -> I/O task.
        let (write_stop_tx, mut write_stop_rx) = mpsc::channel::<IoEvent>(32);

        // Per-connection I/O task
        let id_clone = id.clone();
        let broadcast_tx_clone = broadcast_tx.clone();
        let io_task_handle = tokio::spawn(async move {
            info!("Async I/O task started for connection '{}'.", id_clone);
            let mut buf = [0u8; 256];
            loop {
                // This implicitly awaits concurrently for
                // the write_stop_rx.recv() and conn.read() futures
                tokio::select! {
                    Some(event) = write_stop_rx.recv() => {
                        match event {
                            IoEvent::Write(data) => {
                                debug!("Write: {:?} to connection", data);
                                if let Err(e) = conn.write(&data).await {
                                    error!("Write error on '{}': {:?}", id_clone, e);
                                }
                            },
                            IoEvent::Stop => {
                                info!("Stop received for '{}'. Exiting task.", id_clone);
                                break;
                            },
                        }
                    },
                    result = conn.read(&mut buf) => {
                        match result {
                            Ok(0) => {
                                debug!("Read 0 bytes from '{}'", id_clone);
                            },
                            Ok(n) => {
                                debug!("Read {} bytes from '{}'", n, id_clone);
                                let _ = broadcast_tx_clone.send(buf[..n].to_vec());
                            },
                            Err(e) => {
                                debug!("Read error on '{}': {:?}", id_clone, e);
                                break;
                            },
                        }
                    }
                }
            }
            let _ = conn.disconnect().await;
            info!("Async I/O task ended for '{}'.", id_clone);
        });

        let handle = ConnectionIOHandle {
            io_task_handle,
            write_stop_tx,
            broadcast_tx,
        };
        {
            let mut map = self.inner.lock().await;
            map.insert(id.clone(), handle);
        }

        Ok(ConnectionHandle {
            manager: self.clone(),
            id,
        })
    }

    /// Subscribe to the byte stream of a connection.
    pub async fn subscribe(&self, id: &str) -> Option<broadcast::Receiver<Vec<u8>>> {
        let map = self.inner.lock().await;
        map.get(id).map(|h| h.broadcast_tx.subscribe())
    }

    /// Write bytes to a specific connection by ID.
    pub async fn write_bytes(&self, id: &str, data: &[u8]) -> Result<usize, ConnectionError> {
        let map = self.inner.lock().await;
        if let Some(handle) = map.get(id) {
            debug!("write: {:?}", data);
            handle
                .write_stop_tx
                .send(IoEvent::Write(data.to_vec()))
                .await
                .map_err(|_| ConnectionError::Other("Channel closed".into()))?;
            Ok(data.len())
        } else {
            Err(ConnectionError::Other(format!(
                "No connection with id '{}'",
                id
            )))
        }
    }

    /// Stop a connection.
    pub async fn stop_connection(&self, id: &str) -> Result<(), ConnectionError> {
        let mut map = self.inner.lock().await;
        if let Some(handle) = map.remove(id) {
            let _ = handle.write_stop_tx.send(IoEvent::Stop).await;
            let _ = handle.io_task_handle.await;
            Ok(())
        } else {
            Err(ConnectionError::Other(format!(
                "No connection with id '{}'",
                id
            )))
        }
    }
}

impl ConnectionHandle {
    /// Write bytes using this handle.
    pub async fn write_bytes(&self, data: &[u8]) -> Result<usize, ConnectionError> {
        self.manager.write_bytes(&self.id, data).await
    }

    /// Stop this connection.
    pub async fn stop(self) -> Result<(), ConnectionError> {
        self.manager.stop_connection(&self.id).await
    }
}
