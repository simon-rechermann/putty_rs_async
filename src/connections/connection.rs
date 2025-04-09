use async_trait::async_trait;
use crate::connections::errors::ConnectionError;

/// A trait representing a generic connection (serial, SSH, etc.).
#[async_trait]
pub trait Connection {
    async fn connect(&mut self) -> Result<(), ConnectionError>;
    async fn disconnect(&mut self) -> Result<(), ConnectionError>;

    async fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError>;
}
