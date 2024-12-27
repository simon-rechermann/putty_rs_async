use super::{Connection, ConnectionError};

/// A simple ConnectionManager to create and store connections.
pub struct ConnectionManager;

impl ConnectionManager {
    /// Create a new connection manager.
    pub fn new() -> Self {
        ConnectionManager
    }

    /// Example method to create a connection from some config. 
    /// For now, it's just a placeholder (the real logic might parse e.g. "serial://", "ssh://", etc.).
    pub fn create_connection<T: Connection>(&self, mut conn: T) -> Result<T, ConnectionError> {
        conn.connect()?;
        Ok(conn)
    }

    /// Potentially you could store multiple connections, look them up, etc.
    /// This method is just a placeholder for "destroying" a connection.
    pub fn destroy_connection<T: Connection>(&self, mut conn: T) -> Result<(), ConnectionError> {
        conn.disconnect()
    }
}
