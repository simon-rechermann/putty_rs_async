use tokio_serial::{SerialStream, SerialPortBuilderExt}; // Import SerialPortBuilderExt for open_native_async
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::Duration;
use async_trait::async_trait;
use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;

#[derive(Debug)]
pub struct SerialConnection {
    port_path: String,
    baud_rate: u32,
    inner: Option<SerialStream>,
}

impl SerialConnection {
    pub fn new(port_path: String, baud_rate: u32) -> Self {
        Self {
            port_path,
            baud_rate,
            inner: None,
        }
    }
}

#[async_trait]
impl Connection for SerialConnection {
    async fn connect(&mut self) -> Result<(), ConnectionError> {
        log::info!("Attempting to open serial port: {}", self.port_path);
        let builder = tokio_serial::new(&self.port_path, self.baud_rate)
            .timeout(Duration::from_millis(10));
        match builder.open_native_async() {
            Ok(port) => {
                log::info!("Successfully opened serial port: {}", self.port_path);
                self.inner = Some(port);
                Ok(())
            },
            Err(e) => Err(ConnectionError::from(e))
        }
    }
    
    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if self.inner.is_some() {
            log::info!("Closing serial port: {}", self.port_path);
        }
        self.inner = None;
        Ok(())
    }
    
    async fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(port) = self.inner.as_mut() {
            let bytes_written = port
                .write(data)
                .await
                .map_err(|e| ConnectionError::Other(e.to_string()))?;
            port.flush()
                .await
                .map_err(|e| ConnectionError::Other(e.to_string()))?;
            Ok(bytes_written)
        } else {
            log::error!("Cannot write: serial port not connected!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
    
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if let Some(port) = self.inner.as_mut() {
            let n = port
                .read(buffer)
                .await
                .map_err(|e| ConnectionError::Other(e.to_string()))?;
            Ok(n)
        } else {
            log::error!("Cannot read: serial port not connected!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
}
