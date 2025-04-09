use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;
use log::{debug, error, info};
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

enum IoEvent {
    Write(Vec<u8>),
    Stop,
}

struct ConnectionIOHandle {
    task_handle: tokio::task::JoinHandle<()>,
    tx: mpsc::Sender<IoEvent>,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    manager: ConnectionManager,
    id: String,
}

#[derive(Clone)]
pub struct ConnectionManager {
    inner: Arc<Mutex<HashMap<String, ConnectionIOHandle>>>,
}

impl ConnectionManager {
    /// Create an empty ConnectionManager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a new connection to this ConnectionManager.
    /// - `id`: A unique identifier (e.g. port name or host)
    /// - `conn`: A *not-yet-connected* Connection
    /// - `on_byte`: A callback invoked on each received byte
    pub async fn add_connection(
        &self,
        id: String,
        mut conn: Box<dyn Connection + Send + Unpin>,
        mut on_byte: impl FnMut(u8) + Send + 'static,
    ) -> Result<ConnectionHandle, ConnectionError> {
        // 1) Connect the connection.
        conn.connect().await?;
        
        // 2) Create an mpsc channel for I/O events.
        let (tx, mut rx) = mpsc::channel::<IoEvent>(32);
        let id_clone = id.clone();
        
        // 3) Spawn an async task for the I/O loop.
        let task_handle = tokio::spawn(async move {
            info!("Async I/O task started for connection '{}'.", id_clone);
            let mut buf = [0u8; 256];
            loop {
                tokio::select! {
                    Some(event) = rx.recv() => {
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
                                for &byte in &buf[..n] {
                                    on_byte(byte);
                                }
                            },
                            Err(e) => {
                                debug!("Read error on '{}': {:?}", id_clone, e);
                            },
                        }
                    },
                    else => {
                        break;
                    }
                }
                sleep(Duration::from_millis(5)).await;
            }
            let _ = conn.disconnect().await;
            info!("Async I/O task ended for '{}'.", id_clone);
        });
        
        let handle = ConnectionIOHandle { task_handle, tx: tx.clone() };
        {
            let mut map = self.inner.lock().await;
            map.insert(id.clone(), handle);
        }
        
        Ok(ConnectionHandle {
            manager: self.clone(),
            id,
        })
    }
    
    /// Write bytes to a specific connection by ID.
    pub async fn write_bytes(&self, id: &str, data: &[u8]) -> Result<usize, ConnectionError> {
        let map = self.inner.lock().await;
        if let Some(handle) = map.get(id) {
            debug!("write: {:?}", data);
            handle.tx.send(IoEvent::Write(data.to_vec()))
                .await
                .map_err(|_| ConnectionError::Other("Channel closed".into()))?;
            Ok(data.len())
        } else {
            Err(ConnectionError::Other(format!("No connection with id '{}'", id)))
        }
    }
    
    /// Stop a connection.
    pub async fn stop_connection(&self, id: &str) -> Result<(), ConnectionError> {
        let mut map = self.inner.lock().await;
        if let Some(handle) = map.remove(id) {
            let _ = handle.tx.send(IoEvent::Stop).await;
            let _ = handle.task_handle.await;
            Ok(())
        } else {
            Err(ConnectionError::Other(format!("No connection with id '{}'", id)))
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
