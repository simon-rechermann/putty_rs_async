use serialport::{SerialPort, SerialPortType};
use std::time::Duration;
use log::{info, error};
use ctrlc;

use crate::core::{Connection, ConnectionError};

/// A struct that holds info about our serial connection.
#[derive(Debug)]
pub struct SerialConnection {
    port_path: String,
    baud_rate: u32,
    inner: Option<Box<dyn SerialPort>>,
}

impl SerialConnection {
    pub fn new(port_path: String, baud_rate: u32) -> Self {
        SerialConnection {
            port_path,
            baud_rate,
            inner: None,
        }
    }
}

impl Connection for SerialConnection {
    fn connect(&mut self) -> Result<(), ConnectionError> {
        info!("Attempting to open serial port: {}", self.port_path);

        // `open()` already returns a `Box<dyn SerialPort>`.
        let serial_port = serialport::new(&self.port_path, self.baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()?;

        info!("Successfully opened serial port: {}", self.port_path);

        // No need to wrap in another Box.
        self.inner = Some(serial_port);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if self.inner.is_some() {
            info!("Closing serial port: {}", self.port_path);
        }
        self.inner = None;
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(port) = self.inner.as_mut() {
            let bytes_written = port.write(data).map_err(ConnectionError::from)?;
            port.flush().map_err(ConnectionError::from)?;
            Ok(bytes_written)
        } else {
            error!("Cannot write: serial port not connected!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if let Some(port) = self.inner.as_mut() {
            port.read(buffer).map_err(ConnectionError::from)
        } else {
            error!("Cannot read: serial port not connected!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
}

