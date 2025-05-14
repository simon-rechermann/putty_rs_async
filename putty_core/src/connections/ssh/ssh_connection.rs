use crate::connections::{connection::Connection, errors::ConnectionError};
use async_trait::async_trait;
use log::{error, info};
use ssh2::Session;

use std::io::ErrorKind;
use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    thread,
    time::Duration,
};
use tokio::sync::mpsc;

pub struct SshConnection {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    keyfile:  Option<(PathBuf, Option<String>)>,

    write_tx: Option<mpsc::Sender<Vec<u8>>>,
    read_rx: Option<mpsc::Receiver<Vec<u8>>>,

    leftovers: VecDeque<u8>,
    worker: Option<thread::JoinHandle<()>>,
}

impl SshConnection {
    pub fn new(host: String, port: u16, username: String, password: String) -> Self {
        Self {
            host,
            port,
            username,
            password: Some(password),
            keyfile: None,
            write_tx: None,
            read_rx: None,
            leftovers: VecDeque::new(),
            worker: None,
        }
    }

    /// Constructor for public‑key authentication
    pub fn with_key(
        host: String,
        port: u16,
        username: String,
        private_key: PathBuf,
        passphrase: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            username,
            password: None,
            keyfile: Some((private_key, passphrase)),
            write_tx: None,
            read_rx: None,
            leftovers: VecDeque::new(),
            worker: None,
        }
    }
}

#[async_trait]
impl Connection for SshConnection {
    async fn connect(&mut self) -> Result<(), ConnectionError> {
        let addr = format!("{}:{}", self.host, self.port);
        let username = self.username.clone();
        let password = self.password.clone();
        let keyfile = self.keyfile.clone();

        let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(32);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(32);

        info!("Connecting to SSH server at {}", addr);

        // ---------------- blocking worker -----------------------------
        let worker = thread::spawn(move || {
            // ---- establish session & channel --------------------------
            let tcp = match TcpStream::connect(&addr) {
                Ok(t) => t,
                Err(e) => {
                    error!("TCP connect error: {}", e);
                    return;
                }
            };

            tcp.set_read_timeout(Some(Duration::from_millis(500))).ok();
            tcp.set_write_timeout(Some(Duration::from_millis(500))).ok();

            let mut session = match Session::new() {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create SSH session: {}", e);
                    return;
                }
            };
            session.set_tcp_stream(tcp);
            if let Err(e) = session.handshake() {
                error!("Handshake error: {}", e);
                return;
            }

            // ---- authenticate ----------------------------------------
            let auth_res = if let Some((privkey, phr)) = keyfile {
                session.userauth_pubkey_file(
                    &username,
                    None,                // let libssh2 derive ".pub"
                    &privkey,
                    phr.as_deref(),
                )
            } else if let Some(pw) = password {
                session.userauth_password(&username, &pw)
            } else {
                Err(ssh2::Error::from_errno(ssh2::ErrorCode::Session(-18)))
            };

            if let Err(e) = auth_res {
                error!("Authentication error: {}", e);
                return;
            }
            if !session.authenticated() {
                error!("SSH authentication failed");
                return;
            }

            let mut channel = match session.channel_session() {
                Ok(c) => c,
                Err(e) => {
                    error!("Channel error: {}", e);
                    return;
                }
            };
            channel
                .request_pty("xterm", None, Some((80, 24, 0, 0)))
                .ok();
            channel.shell().ok();
            session.set_blocking(false);

            info!("SSH connection established");

            // ---- I/O loop --------------------------------------------
            let mut buf = [0u8; 1024];

            loop {
                // outgoing
                while let Ok(pkt) = write_rx.try_recv() {
                    if let Err(e) = channel.write_all(&pkt) {
                        error!("SSH write error: {}", e);
                        return;
                    }
                    channel.flush().ok();
                }

                // incoming
                match channel.read(&mut buf) {
                    Ok(0) => {} // nothing
                    Ok(n) => {
                        if read_tx.blocking_send(buf[..n].to_vec()).is_err() {
                            return; // receiver gone
                        }
                    }

                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => { /* WouldBlock */ }
                    Err(e) => {
                        error!("SSH read error: {}", e);
                        return;
                    }
                }

                thread::sleep(Duration::from_millis(2));
            }
        });
        // --------------------------------------------------------------

        self.write_tx = Some(write_tx);
        self.read_rx = Some(read_rx);
        self.worker = Some(worker);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        self.write_tx = None; // tell worker to exit
        if let Some(jh) = self.worker.take() {
            let _ = jh.join();
        }
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        match &self.write_tx {
            Some(tx) => {
                tx.send(data.to_vec())
                    .await
                    .map_err(|_| ConnectionError::Other("SSH write channel closed".into()))?;
                Ok(data.len())
            }
            None => Err(ConnectionError::Other("Not connected".into())),
        }
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        // serve leftovers first
        if !self.leftovers.is_empty() {
            let n = std::cmp::min(buffer.len(), self.leftovers.len());
            for (dst, src) in buffer.iter_mut().take(n).zip(self.leftovers.drain(..n)) {
                *dst = src;
            }
            return Ok(n);
        }

        match &mut self.read_rx {
            Some(rx) => match rx.recv().await {
                Some(mut chunk) => {
                    let n = std::cmp::min(buffer.len(), chunk.len());
                    buffer[..n].copy_from_slice(&chunk[..n]);
                    if chunk.len() > n {
                        self.leftovers.extend(chunk.split_off(n));
                    }
                    Ok(n)
                }
                None => Err(ConnectionError::Other("SSH connection closed".into())),
            },
            None => Err(ConnectionError::Other("Not connected".into())),
        }
    }
}
