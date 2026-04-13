use crate::expose::auth::RuntimeAuth;
use crate::expose::types::ExposeConfig;
use anyhow::anyhow;
use russh::client::{self, AuthResult, Msg};
use russh::keys::{self, Algorithm, HashAlg, PublicKey, key::PrivateKeyWithHashAlg};
use russh::{Channel, Error as RusshError};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub struct SshTunnel {
    handle: client::Handle<ClientHandler>,
    forwarded_connections: mpsc::Receiver<ForwardedConnection>,
}

pub struct SshForwardSession {
    handle: client::Handle<ClientHandler>,
}

#[derive(Debug)]
pub enum ConnectError {
    Transport(anyhow::Error),
    InvalidPrivateKey(anyhow::Error),
    AuthenticationRejected(anyhow::Error),
}

#[derive(Debug)]
pub enum ForwardError {
    RequestDenied {
        remote_host: String,
        remote_port: u16,
    },
    Request {
        remote_host: String,
        remote_port: u16,
        source: RusshError,
    },
}

pub struct ForwardedConnection {
    pub channel: russh::ChannelStream<Msg>,
    pub connected_address: String,
    pub connected_port: u32,
    pub originator_address: String,
    pub originator_port: u32,
}

#[derive(Clone)]
struct ClientHandler {
    forwarded_connections: mpsc::Sender<ForwardedConnection>,
}

#[derive(Debug)]
struct SshClientError(anyhow::Error);

impl std::fmt::Display for SshClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SshClientError {}

impl From<RusshError> for SshClientError {
    fn from(error: RusshError) -> Self {
        Self(error.into())
    }
}

impl From<anyhow::Error> for SshClientError {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}

impl ClientHandler {
    fn new(forwarded_connections: mpsc::Sender<ForwardedConnection>) -> Self {
        Self {
            forwarded_connections,
        }
    }
}

impl client::Handler for ClientHandler {
    type Error = SshClientError;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        warn!(
            fingerprint = %server_public_key.fingerprint(HashAlg::Sha256),
            algorithm = %server_public_key.algorithm(),
            "accepting SSH host key without verification"
        );

        Ok(true)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let forwarded_connection = ForwardedConnection {
            channel: channel.into_stream(),
            connected_address: connected_address.to_string(),
            connected_port,
            originator_address: originator_address.to_string(),
            originator_port,
        };

        self.forwarded_connections
            .send(forwarded_connection)
            .await
            .map_err(|_| anyhow!("forwarded connection queue closed"))?;

        Ok(())
    }
}

impl Debug for ClientHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientHandler").finish_non_exhaustive()
    }
}

impl SshTunnel {
    pub async fn connect(config: &ExposeConfig, auth: &RuntimeAuth) -> Result<Self, ConnectError> {
        let (handle, forwarded_connections) =
            connect_handle(&config.server, &config.user, auth).await?;

        Ok(Self {
            handle,
            forwarded_connections,
        })
    }

    pub async fn request_remote_forward(
        &self,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<u32, ForwardError> {
        let bound_port = self
            .handle
            .tcpip_forward(remote_host, u32::from(remote_port))
            .await
            .map_err(|error| match error {
                RusshError::RequestDenied => ForwardError::RequestDenied {
                    remote_host: remote_host.to_string(),
                    remote_port,
                },
                other => ForwardError::Request {
                    remote_host: remote_host.to_string(),
                    remote_port,
                    source: other,
                },
            })?;

        Ok(bound_port)
    }

    pub async fn next_forwarded_connection(&mut self) -> Option<ForwardedConnection> {
        self.forwarded_connections.recv().await
    }
}

impl SshForwardSession {
    pub async fn connect(
        server: &str,
        user: &str,
        auth: &RuntimeAuth,
    ) -> Result<Self, ConnectError> {
        let (handle, _forwarded_connections) = connect_handle(server, user, auth).await?;
        Ok(Self { handle })
    }

    pub async fn open_direct_tcpip(
        &self,
        target_host: &str,
        target_port: u16,
        originator_address: &str,
        originator_port: u16,
    ) -> anyhow::Result<russh::ChannelStream<Msg>> {
        let channel = self
            .handle
            .channel_open_direct_tcpip(
                target_host,
                u32::from(target_port),
                originator_address,
                u32::from(originator_port),
            )
            .await
            .map_err(|error| {
                anyhow!(
                    "failed to open direct TCP/IP channel to {}:{}: {}",
                    target_host,
                    target_port,
                    error
                )
            })?;

        Ok(channel.into_stream())
    }

    pub fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }
}

async fn connect_handle(
    server: &str,
    user: &str,
    auth: &RuntimeAuth,
) -> Result<
    (
        client::Handle<ClientHandler>,
        mpsc::Receiver<ForwardedConnection>,
    ),
    ConnectError,
> {
    let ssh_config = Arc::new(client::Config::default());
    let (forwarded_connections_tx, forwarded_connections) = mpsc::channel(64);
    let handler = ClientHandler::new(forwarded_connections_tx);

    let mut handle = client::connect(ssh_config, server, handler)
        .await
        .map_err(|error| {
            ConnectError::Transport(anyhow!(
                "failed to connect to SSH server {}: {}",
                server,
                error.0
            ))
        })?;

    authenticate(&mut handle, server, user, auth).await?;

    info!(server = %server, user = %user, "SSH session established");

    Ok((handle, forwarded_connections))
}

async fn authenticate(
    handle: &mut client::Handle<ClientHandler>,
    server: &str,
    user: &str,
    auth: &RuntimeAuth,
) -> Result<(), ConnectError> {
    let auth_result = match auth {
        RuntimeAuth::Password(password) => {
            info!(user = %user, "authenticating with SSH password");
            handle
                .authenticate_password(user, password)
                .await
                .map_err(|error| {
                    ConnectError::Transport(anyhow!(
                        "SSH password authentication failed for {}@{}: {}",
                        user,
                        server,
                        error
                    ))
                })?
        }
        RuntimeAuth::PrivateKeyFile(path) => {
            info!(user = %user, key = %path.display(), "authenticating with SSH private key");
            let private_key = keys::load_secret_key(path, None).map_err(|error| {
                ConnectError::InvalidPrivateKey(anyhow!(
                    "failed to load private key {}: {}",
                    path.display(),
                    error
                ))
            })?;
            let hash_alg = select_hash_alg(handle, &private_key)
                .await
                .map_err(ConnectError::Transport)?;
            let private_key = PrivateKeyWithHashAlg::new(Arc::new(private_key), hash_alg);

            handle
                .authenticate_publickey(user, private_key)
                .await
                .map_err(|error| {
                    ConnectError::Transport(anyhow!(
                        "SSH public key authentication failed for {}@{} using {}: {}",
                        user,
                        server,
                        path.display(),
                        error
                    ))
                })?
        }
    };

    ensure_auth_success(auth_result, server, user, auth)
}

async fn select_hash_alg(
    handle: &client::Handle<ClientHandler>,
    private_key: &keys::PrivateKey,
) -> anyhow::Result<Option<HashAlg>> {
    if !matches!(private_key.algorithm(), Algorithm::Rsa { .. }) {
        return Ok(None);
    }

    let hash_alg = handle
        .best_supported_rsa_hash()
        .await
        .map_err(|error| anyhow!("failed to determine RSA hash algorithm: {}", error))?
        .flatten()
        .or(Some(HashAlg::Sha256));

    debug!(
        ?hash_alg,
        "selected RSA hash algorithm for SSH authentication"
    );

    Ok(hash_alg)
}

fn ensure_auth_success(
    auth_result: AuthResult,
    server: &str,
    user: &str,
    auth: &RuntimeAuth,
) -> Result<(), ConnectError> {
    if auth_result.success() {
        return Ok(());
    }

    let error = match auth {
        RuntimeAuth::Password(_) => anyhow!(
            "SSH authentication was rejected by the server for {}@{} using password",
            user,
            server
        ),
        RuntimeAuth::PrivateKeyFile(path) => anyhow!(
            "SSH authentication was rejected by the server for {}@{} using key {}",
            user,
            server,
            path.display()
        ),
    };

    Err(ConnectError::AuthenticationRejected(error))
}
