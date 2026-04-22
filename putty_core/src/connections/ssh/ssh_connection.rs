use crate::connections::{connection::Connection, errors::ConnectionError};
use async_trait::async_trait;
use log::{debug, info};
use russh::client::{self, AuthResult, Handle};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg};
use russh::{Channel, ChannelMsg, Disconnect};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

impl From<russh::Error> for ConnectionError {
    fn from(err: russh::Error) -> Self {
        ConnectionError::Other(format!("SSH: {err}"))
    }
}

struct SshClient;

impl client::Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Host-key verification lives in the TODO list; accept everything for now.
        Ok(true)
    }
}

pub struct SshConnection {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    keyfile: Option<(PathBuf, Option<String>)>,

    session: Option<Handle<SshClient>>,
    channel: Option<Channel<client::Msg>>,
    leftovers: VecDeque<u8>,
}

impl SshConnection {
    pub fn new(host: String, port: u16, username: String, password: String) -> Self {
        Self {
            host,
            port,
            username,
            password: Some(password),
            keyfile: None,
            session: None,
            channel: None,
            leftovers: VecDeque::new(),
        }
    }

    /// Constructor for public-key authentication.
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
            session: None,
            channel: None,
            leftovers: VecDeque::new(),
        }
    }
}

#[async_trait]
impl Connection for SshConnection {
    async fn connect(&mut self) -> Result<(), ConnectionError> {
        let addr = format!("{}:{}", self.host, self.port);
        info!("Connecting to SSH server at {addr}");

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(60)),
            ..Default::default()
        });

        let mut session = client::connect(config, addr, SshClient).await?;

        let auth_result: AuthResult = if let Some((key_path, passphrase)) = self.keyfile.clone() {
            let key = load_secret_key(&key_path, passphrase.as_deref())
                .map_err(|e| ConnectionError::Other(format!("SSH key load error: {e}")))?;
            let rsa_hash = session.best_supported_rsa_hash().await?.flatten();
            session
                .authenticate_publickey(
                    self.username.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), rsa_hash),
                )
                .await?
        } else if let Some(pw) = self.password.clone() {
            session
                .authenticate_password(self.username.clone(), pw)
                .await?
        } else {
            return Err(ConnectionError::Other(
                "No SSH authentication method configured".into(),
            ));
        };

        if !auth_result.success() {
            return Err(ConnectionError::Other("SSH authentication failed".into()));
        }

        let channel = session.channel_open_session().await?;
        channel
            .request_pty(false, "xterm", 80, 24, 0, 0, &[])
            .await?;
        channel.request_shell(false).await?;

        info!("SSH connection established");
        self.session = Some(session);
        self.channel = Some(channel);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if let Some(channel) = self.channel.take() {
            let _ = channel.close().await;
        }
        if let Some(session) = self.session.take() {
            let _ = session
                .disconnect(Disconnect::ByApplication, "bye", "en")
                .await;
        }
        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<usize, ConnectionError> {
        let channel = self
            .channel
            .as_ref()
            .ok_or_else(|| ConnectionError::Other("Not connected".into()))?;
        channel.data(data).await?;
        Ok(data.len())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ConnectionError> {
        if !self.leftovers.is_empty() {
            let n = std::cmp::min(buffer.len(), self.leftovers.len());
            for (dst, src) in buffer.iter_mut().take(n).zip(self.leftovers.drain(..n)) {
                *dst = src;
            }
            return Ok(n);
        }

        let channel = self
            .channel
            .as_mut()
            .ok_or_else(|| ConnectionError::Other("Not connected".into()))?;

        loop {
            match channel.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    return Ok(copy_with_leftovers(&data, buffer, &mut self.leftovers));
                }
                Some(ChannelMsg::ExtendedData { data, .. }) => {
                    return Ok(copy_with_leftovers(&data, buffer, &mut self.leftovers));
                }
                Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                    return Err(ConnectionError::Other("SSH connection closed".into()));
                }
                Some(other) => {
                    debug!("Ignoring SSH channel message: {other:?}");
                }
            }
        }
    }
}

fn copy_with_leftovers(src: &[u8], buf: &mut [u8], leftovers: &mut VecDeque<u8>) -> usize {
    let n = std::cmp::min(buf.len(), src.len());
    buf[..n].copy_from_slice(&src[..n]);
    if src.len() > n {
        leftovers.extend(src[n..].iter().copied());
    }
    n
}
