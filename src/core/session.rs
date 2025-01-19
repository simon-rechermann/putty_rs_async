use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::io::{self, Write};

use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;
use crate::core::ConnectionManager;
use log::debug;

/// A struct that represents a running session:
/// - Spawns a background thread to read from the connection
/// - Invokes a user-provided callback for each byte read
/// - Exposes `write_bytes` for sending data, and `stop` to disconnect
pub struct Session {
    manager: ConnectionManager,
    connection: Arc<Mutex<Box<dyn Connection + Send>>>,
    stop_flag: Arc<AtomicBool>,
    reader_thread: Option<thread::JoinHandle<()>>,

    // A callback used for new bytes
    on_byte: Arc<Mutex<dyn FnMut(u8) + Send>>,
}

impl Session {
    /// Create (but not start) a Session. `on_byte` is a closure that will run for each byte read.
    pub fn new(
        manager: ConnectionManager,
        connection: Box<dyn Connection + Send>,
        on_byte: impl FnMut(u8) + Send + 'static,
    ) -> Self {
        Self {
            manager,
            connection: Arc::new(Mutex::new(connection)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            reader_thread: None,
            on_byte: Arc::new(Mutex::new(on_byte)),
        }
    }

    /// Start the background thread to read from the connection.
    pub fn start(&mut self) {
        let stop_clone = self.stop_flag.clone();
        let conn_clone = self.connection.clone();
        let callback_clone = self.on_byte.clone();

        let handle = thread::spawn(move || {
            let mut buf = [0u8; 256];
            while !stop_clone.load(Ordering::SeqCst) {
                {
                    let mut conn = conn_clone.lock().unwrap();
                    match conn.read(&mut buf) {
                        Ok(0) => {
                            debug!("Ok(0) Not data");
                            // no data
                        }
                        Ok(n) => {
                            debug!("Ok(n): {} bytes arrived", n);
                            let data = &buf[..n];
                            let mut cb = callback_clone.lock().unwrap();
                            for &byte in data {
                                cb(byte);
                            }
                            io::stdout().flush().ok();
                        }
                        Err(ConnectionError::IoError(ref io_err))
                            if io_err.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            // no data during that interval
                            debug!("Timeout error");
                        }
                        Err(e) => {
                            debug!("Read error: {:?}", e);
                            // Decide if you want to break or keep reading
                        }
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
            debug!("Reader thread stopped.");
        });
        self.reader_thread = Some(handle);
    }

    /// Write data to the connection from any caller (CLI or GUI).
    pub fn write_bytes(&self, data: &[u8]) -> Result<usize, ConnectionError> {
        let mut conn = self.connection.lock().unwrap();
        conn.write(data)
    }

    /// Stop reading + disconnect from the device
    pub fn stop(&mut self) -> Result<(), ConnectionError> {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }

        // Disconnect
        let mut conn = self.connection.lock().unwrap();
        self.manager.destroy_connection(&mut **conn)?;

        Ok(())
    }
}
