use log::{debug, error, info};
use ssh2::{Channel, Session};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;

/// A blocking SSH connection using the ssh2 library.
pub struct SshConnection {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    inner: Option<Channel>,
    session: Option<Session>,
}

impl SshConnection {
    pub fn new(host: String, port: u16, username: String, password: String) -> Self {
        SshConnection {
            host,
            port,
            username,
            password,
            inner: None,
            session: None,
        }
    }
}

impl Connection for SshConnection {
    fn connect(&mut self) -> Result<(), ConnectionError> {
        let address = format!("{}:{}", self.host, self.port);
        info!("Connecting to SSH server at {}", address);

        let tcp = TcpStream::connect(&address)
            .map_err(|e| ConnectionError::Other(format!("TCP connect error: {}", e)))?;
        tcp.set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| ConnectionError::Other(format!("Set read timeout error: {}", e)))?;
        tcp.set_write_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| ConnectionError::Other(format!("Set write timeout error: {}", e)))?;

        let mut session = Session::new()
            .map_err(|e| ConnectionError::Other(format!("Failed to create SSH session: {}", e)))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| ConnectionError::Other(format!("Handshake error: {}", e)))?;
        session
            .userauth_password(&self.username, &self.password)
            .map_err(|e| ConnectionError::Other(format!("Authentication error: {}", e)))?;

        if !session.authenticated() {
            return Err(ConnectionError::Other("SSH authentication failed".into()));
        }

        // Create the channel while still in blocking mode.
        let mut channel = session
            .channel_session()
            .map_err(|e| ConnectionError::Other(format!("Channel session error: {}", e)))?;
        channel
            .request_pty("xterm", None, Some((80, 24, 0, 0)))
            .map_err(|e| ConnectionError::Other(format!("Request pty error: {}", e)))?;
        channel
            .shell()
            .map_err(|e| ConnectionError::Other(format!("Shell error: {}", e)))?;

        // Now switch the session (and associated channel) to nonblocking mode.
        session.set_blocking(false);

        self.inner = Some(channel);
        self.session = Some(session);
        info!("SSH connection established and shell channel opened.");
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if let Some(mut channel) = self.inner.take() {
            channel
                .close()
                .map_err(|e| ConnectionError::Other(format!("Close channel error: {}", e)))?;
            channel
                .wait_close()
                .map_err(|e| ConnectionError::Other(format!("Wait close error: {}", e)))?;
            info!("SSH channel closed.");
        }
        self.session = None;
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(ref mut channel) = self.inner {
            // Drain any pending incoming data.
            let mut dummy = [0u8; 256];
            loop {
                match channel.read(&mut dummy) {
                    Ok(0) => {
                        // No more data.
                        break;
                    }
                    Ok(n) => {
                        debug!("Drained {} bytes from incoming flow", n);
                        // Continue draining.
                    }
                    Err(e) if e.to_string().contains("WouldBlock") => {
                        // Nothing more to drain.
                        break;
                    }
                    Err(e) => {
                        debug!("Drain error (ignored): {:?}", e);
                        break;
                    }
                }
            }

            // Now write the data.
            let bytes_written = channel
                .write(data)
                .map_err(|e| ConnectionError::Other(format!("Write error: {}", e)))?;
            channel
                .flush()
                .map_err(|e| ConnectionError::Other(format!("Flush error: {}", e)))?;
            debug!("Wrote {} bytes ({:?}) to ssh server", bytes_written, data);
            Ok(bytes_written)
        } else {
            error!("SSH connection not established!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if let Some(ref mut channel) = self.inner {
            channel
                .read(buffer)
                .map_err(|e| ConnectionError::Other(format!("Read error: {}", e)))
        } else {
            error!("SSH connection not established!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
}
