use crate::expose::auth::RuntimeAuth;
use crate::expose::types::ExposeConfig;
use anyhow::{Context, anyhow, bail};
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
    pub async fn connect(config: &ExposeConfig, auth: &RuntimeAuth) -> anyhow::Result<Self> {
        let ssh_config = Arc::new(client::Config::default());
        let (forwarded_connections_tx, forwarded_connections) = mpsc::channel(64);
        let handler = ClientHandler::new(forwarded_connections_tx);

        let mut handle = client::connect(ssh_config, &config.server, handler)
            .await
            .map_err(|error| error.0)
            .with_context(|| format!("failed to connect to SSH server {}", config.server))?;

        authenticate(&mut handle, config, auth).await?;

        info!(server = %config.server, user = %config.user, "SSH session established");

        Ok(Self {
            handle,
            forwarded_connections,
        })
    }

    pub async fn request_remote_forward(
        &self,
        remote_host: &str,
        remote_port: u16,
    ) -> anyhow::Result<u32> {
        let bound_port = self
            .handle
            .tcpip_forward(remote_host, u32::from(remote_port))
            .await
            .with_context(|| {
                format!(
                    "failed to register remote forward on {}:{}",
                    remote_host, remote_port
                )
            })?;

        Ok(bound_port)
    }

    pub async fn next_forwarded_connection(&mut self) -> Option<ForwardedConnection> {
        self.forwarded_connections.recv().await
    }
}

async fn authenticate(
    handle: &mut client::Handle<ClientHandler>,
    config: &ExposeConfig,
    auth: &RuntimeAuth,
) -> anyhow::Result<()> {
    let auth_result = match auth {
        RuntimeAuth::Password(password) => {
            info!(user = %config.user, "authenticating with SSH password");
            handle
                .authenticate_password(&config.user, password)
                .await
                .context("SSH password authentication failed")?
        }
        RuntimeAuth::PrivateKeyFile(path) => {
            info!(user = %config.user, key = %path.display(), "authenticating with SSH private key");
            let private_key = keys::load_secret_key(path, None)
                .with_context(|| format!("failed to load private key {}", path.display()))?;
            let hash_alg = select_hash_alg(handle, &private_key).await?;
            let private_key = PrivateKeyWithHashAlg::new(Arc::new(private_key), hash_alg);

            handle
                .authenticate_publickey(&config.user, private_key)
                .await
                .context("SSH public key authentication failed")?
        }
    };

    ensure_auth_success(auth_result)
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
        .context("failed to determine RSA hash algorithm")?
        .flatten()
        .or(Some(HashAlg::Sha256));

    debug!(
        ?hash_alg,
        "selected RSA hash algorithm for SSH authentication"
    );

    Ok(hash_alg)
}

fn ensure_auth_success(auth_result: AuthResult) -> anyhow::Result<()> {
    if auth_result.success() {
        return Ok(());
    }

    bail!("SSH authentication was rejected by the server")
}
