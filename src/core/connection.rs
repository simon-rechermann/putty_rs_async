use super::errors::ConnectionError;

/// A trait representing a generic connection (serial, SSH, etc.).
pub trait Connection {
    fn connect(&mut self) -> Result<(), ConnectionError>;
    fn disconnect(&mut self) -> Result<(), ConnectionError>;

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError>;
}
