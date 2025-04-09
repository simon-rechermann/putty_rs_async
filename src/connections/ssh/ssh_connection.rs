use async_trait::async_trait;
use log::{debug, error, info};
use ssh2::{Channel, Session};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use crate::connections::connection::Connection;
use crate::connections::errors::ConnectionError;

pub struct SshConnection {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    inner: Option<Arc<Mutex<Channel>>>,
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

#[async_trait]
impl Connection for SshConnection {
    async fn connect(&mut self) -> Result<(), ConnectionError> {
        let host = self.host.clone();
        let port = self.port;
        let username = self.username.clone();
        let password = self.password.clone();
        
        info!("Connecting to SSH server at {}:{}", host, port);
        
        let result = task::spawn_blocking(move || {
            let address = format!("{}:{}", host, port);
            let tcp = TcpStream::connect(&address)
                .map_err(|e| ConnectionError::Other(format!("TCP connect error: {}", e)))?;
            tcp.set_read_timeout(Some(Duration::from_millis(500)))
                .map_err(|e| ConnectionError::Other(format!("Set read timeout error: {}", e)))?;
            tcp.set_write_timeout(Some(Duration::from_millis(500)))
                .map_err(|e| ConnectionError::Other(format!("Set write timeout error: {}", e)))?;
            
            let mut session = Session::new()
                .map_err(|e| ConnectionError::Other(format!("Failed to create SSH session: {}", e)))?;
            session.set_tcp_stream(tcp);
            session.handshake()
                .map_err(|e| ConnectionError::Other(format!("Handshake error: {}", e)))?;
            session.userauth_password(&username, &password)
                .map_err(|e| ConnectionError::Other(format!("Authentication error: {}", e)))?;
            
            if !session.authenticated() {
                return Err(ConnectionError::Other("SSH authentication failed".into()));
            }
            
            let mut channel = session.channel_session()
                .map_err(|e| ConnectionError::Other(format!("Channel session error: {}", e)))?;
            channel.request_pty("xterm", None, Some((80, 24, 0, 0)))
                .map_err(|e| ConnectionError::Other(format!("Request pty error: {}", e)))?;
            channel.shell()
                .map_err(|e| ConnectionError::Other(format!("Shell error: {}", e)))?;
            
            session.set_blocking(false);
            
            Ok((channel, session))
        }).await.map_err(|e| ConnectionError::Other(format!("Join error: {}", e)))?;
        
        match result {
            Ok((channel, session)) => {
                self.inner = Some(Arc::new(Mutex::new(channel)));
                self.session = Some(session);
                info!("SSH connection established and shell channel opened.");
                Ok(())
            }
            Err(e) => Err(e)
        }
    }
    
    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if let Some(inner) = self.inner.take() {
            let res = task::spawn_blocking(move || {
                let mut channel = inner.blocking_lock();
                channel.close()
                    .map_err(|e| ConnectionError::Other(format!("Close channel error: {}", e)))?;
                channel.wait_close()
                    .map_err(|e| ConnectionError::Other(format!("Wait close error: {}", e)))?;
                Ok(())
            }).await.map_err(|e| ConnectionError::Other(format!("Join error: {}", e)))?;
            res?;
            info!("SSH channel closed.");
        }
        self.session = None;
        Ok(())
    }
    
    async fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(inner) = &self.inner {
            let data_vec = data.to_vec();
            let inner_clone = inner.clone();
            let bytes_written = task::spawn_blocking(move || {
                let mut channel = inner_clone.blocking_lock();
                // Drain any pending incoming data.
                let mut dummy = [0u8; 256];
                loop {
                    match channel.read(&mut dummy) {
                        Ok(0) => break,
                        Ok(n) => {
                            debug!("Drained {} bytes from incoming flow", n);
                        },
                        Err(e) if e.to_string().contains("WouldBlock") => break,
                        Err(e) => {
                            debug!("Drain error (ignored): {:?}", e);
                            break;
                        }
                    }
                }
                let bytes_written = channel.write(&data_vec)
                    .map_err(|e| ConnectionError::Other(format!("Write error: {}", e)))?;
                channel.flush()
                    .map_err(|e| ConnectionError::Other(format!("Flush error: {}", e)))?;
                Ok(bytes_written)
            }).await.map_err(|e| ConnectionError::Other(format!("Join error: {}", e)))?;
            bytes_written
        } else {
            error!("SSH connection not established!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
    
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if let Some(inner) = &self.inner {
            let inner_clone = inner.clone();
            let result = task::spawn_blocking(move || {
                let mut channel = inner_clone.blocking_lock();
                channel.read(buffer)
                    .map_err(|e| ConnectionError::Other(format!("Read error: {}", e)))
            }).await.map_err(|e| ConnectionError::Other(format!("Join error: {}", e)))?;
            result
        } else {
            error!("SSH connection not established!");
            Err(ConnectionError::Other("Not connected".into()))
        }
    }
}
