use std::fmt::{self, Display};

/// A central error enum for connection-related errors.
#[derive(Debug)]
pub enum ConnectionError {
    IoError(std::io::Error),
    PortError(String),
    Other(String),
}

/// Convert from std::io::Error.
impl From<std::io::Error> for ConnectionError {
    fn from(err: std::io::Error) -> ConnectionError {
        ConnectionError::IoError(err)
    }
}

/// Convert from serialport::Error.
/// Without this, `map_err(ConnectionError::from)` won't work when using `serialport`.
impl From<serialport::Error> for ConnectionError {
    fn from(err: serialport::Error) -> Self {
        ConnectionError::PortError(err.to_string())
    }
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionError::IoError(e) => write!(f, "IO error: {}", e),
            ConnectionError::PortError(msg) => write!(f, "Port error: {}", msg),
            ConnectionError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl std::error::Error for ConnectionError {}
