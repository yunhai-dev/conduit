use crate::cli::ForwardArgs;
use crate::error::ConduitError;
use crate::expose::auth::{RuntimeAuth, build_auth};
use crate::expose::forwarder;
use crate::expose::ssh_client::{ConnectError, SshForwardSession};
use crate::expose::types::{AuthConfig, parse_server_addr};
use crate::tunnel_registry;
use anyhow::anyhow;
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, watch};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct ForwardConfig {
    pub server: String,
    pub user: String,
    pub bind_addr: IpAddr,
    pub mappings: Vec<ForwardMapping>,
    pub auth: AuthConfig,
    pub daemon: bool,
    pub log_file: Option<PathBuf>,
    pub insecure_accept_host_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardMapping {
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForwardRemoteSpec {
    user: Option<String>,
    server_host: String,
    server_port: u16,
}

impl ForwardRemoteSpec {
    fn parse(input: &str) -> Result<Self, ConduitError> {
        let (user, remainder) = match input.split_once('@') {
            Some((user, remainder)) if !user.is_empty() && !remainder.is_empty() => {
                (Some(user.to_string()), remainder)
            }
            Some(_) => return Err(ConduitError::InvalidForwardRemote(input.to_string())),
            None => (None, input),
        };

        if remainder.starts_with('[') {
            let end = remainder
                .find(']')
                .ok_or_else(|| ConduitError::InvalidForwardRemote(input.to_string()))?;
            let host = &remainder[..=end];
            let rest = &remainder[end + 1..];
            let server_port = if rest.is_empty() {
                22
            } else {
                let port = rest
                    .strip_prefix(':')
                    .ok_or_else(|| ConduitError::InvalidForwardRemote(input.to_string()))?;
                port.parse::<u16>()
                    .map_err(|_| ConduitError::InvalidForwardRemote(input.to_string()))?
            };

            return Ok(Self {
                user,
                server_host: host.to_string(),
                server_port,
            });
        }

        let (server_host, server_port) = match remainder.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() && !host.contains(':') => {
                let parsed = port
                    .parse::<u16>()
                    .map_err(|_| ConduitError::InvalidForwardRemote(input.to_string()))?;
                (host.to_string(), parsed)
            }
            Some(_) => return Err(ConduitError::InvalidForwardRemote(input.to_string())),
            None => (remainder.to_string(), 22),
        };

        if server_host.trim().is_empty() {
            return Err(ConduitError::InvalidForwardRemote(input.to_string()));
        }

        Ok(Self {
            user,
            server_host,
            server_port,
        })
    }
}

impl ForwardMapping {
    pub fn parse(input: &str) -> Result<Self, ConduitError> {
        let (local_port, remainder) = input
            .split_once(':')
            .ok_or_else(|| ConduitError::InvalidForwardLocalSpec(input.to_string()))?;
        let (target_host, target_port) = remainder
            .rsplit_once(':')
            .ok_or_else(|| ConduitError::InvalidForwardLocalSpec(input.to_string()))?;

        if target_host.trim().is_empty() || target_host.contains(char::is_whitespace) {
            return Err(ConduitError::InvalidForwardLocalSpec(input.to_string()));
        }

        let local_port = local_port
            .parse::<u16>()
            .map_err(|_| ConduitError::InvalidForwardLocalSpec(input.to_string()))?;
        let target_port = target_port
            .parse::<u16>()
            .map_err(|_| ConduitError::InvalidForwardLocalSpec(input.to_string()))?;

        Ok(Self {
            local_port,
            target_host: target_host.to_string(),
            target_port,
        })
    }

    pub fn local_addr(&self, bind_addr: IpAddr) -> SocketAddr {
        SocketAddr::new(bind_addr, self.local_port)
    }

    pub fn target_addr(&self) -> String {
        format!("{}:{}", self.target_host, self.target_port)
    }

    pub fn encoded_spec(&self) -> String {
        format!(
            "{}:{}:{}",
            self.local_port, self.target_host, self.target_port
        )
    }
}

impl TryFrom<ForwardArgs> for ForwardConfig {
    type Error = anyhow::Error;

    fn try_from(args: ForwardArgs) -> Result<Self, Self::Error> {
        if args.local.is_empty() {
            return Err(ConduitError::MissingForwardLocalSpec.into());
        }

        let remote = ForwardRemoteSpec::parse(&args.remote)?;
        let auth = match (args.password, args.key) {
            (Some(password), None) => AuthConfig::Password(password),
            (None, Some(path)) => {
                if !path.exists() {
                    return Err(ConduitError::MissingPrivateKey(path).into());
                }
                AuthConfig::PrivateKey(path)
            }
            _ => return Err(ConduitError::InvalidAuthConfig.into()),
        };

        let user = match (remote.user.as_ref(), args.user) {
            (Some(remote_user), Some(cli_user)) if remote_user != &cli_user => {
                return Err(ConduitError::ConflictingConnectUser {
                    remote_user: remote_user.clone(),
                    cli_user,
                }
                .into());
            }
            (Some(remote_user), Some(_)) => remote_user.clone(),
            (Some(remote_user), None) => remote_user.clone(),
            (None, Some(cli_user)) => cli_user,
            (None, None) => return Err(ConduitError::MissingConnectUser.into()),
        };

        let bind_addr = if args.gateway {
            IpAddr::from([0, 0, 0, 0])
        } else {
            args.bind
                .as_deref()
                .unwrap_or("127.0.0.1")
                .parse::<IpAddr>()
                .map_err(|_| {
                    ConduitError::InvalidBindAddress(
                        args.bind.unwrap_or_else(|| "127.0.0.1".to_string()),
                    )
                })?
        };

        let mut seen_binds = HashSet::new();
        let mut mappings = Vec::with_capacity(args.local.len());
        for spec in args.local {
            let mapping = ForwardMapping::parse(&spec)?;
            let bind = mapping.local_addr(bind_addr).to_string();
            if !seen_binds.insert(bind.clone()) {
                return Err(ConduitError::DuplicateForwardBind(bind).into());
            }
            mappings.push(mapping);
        }

        let server = format!("{}:{}", remote.server_host, remote.server_port);
        parse_server_addr(&server)?;

        if !args.insecure_accept_host_key {
            return Err(ConduitError::MissingHostKeyPolicy.into());
        }

        Ok(Self {
            server,
            user,
            bind_addr,
            mappings,
            auth,
            daemon: args.daemon,
            log_file: args.log_file,
            insecure_accept_host_key: args.insecure_accept_host_key,
        })
    }
}

const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);
const SESSION_POLL_INTERVAL: Duration = Duration::from_secs(1);

type SharedSession = Arc<Mutex<SshForwardSession>>;

#[derive(Debug)]
enum SessionFailure {
    Retryable {
        error: anyhow::Error,
        established: bool,
    },
    Terminal(anyhow::Error),
}

#[derive(Debug)]
struct ReconnectBackoff {
    next_delay: Duration,
}

impl ReconnectBackoff {
    fn new() -> Self {
        Self {
            next_delay: INITIAL_RECONNECT_DELAY,
        }
    }

    fn reset(&mut self) {
        self.next_delay = INITIAL_RECONNECT_DELAY;
    }

    fn next_delay(&mut self) -> Duration {
        let delay = self.next_delay;
        let next = self
            .next_delay
            .checked_mul(2)
            .unwrap_or(MAX_RECONNECT_DELAY);
        self.next_delay = std::cmp::min(next, MAX_RECONNECT_DELAY);
        delay
    }
}

struct ListenerBinding {
    mapping: ForwardMapping,
    listener: TcpListener,
}

pub async fn run(config: ForwardConfig, tunnel_id: Option<String>) -> anyhow::Result<()> {
    info!(
        server = %config.server,
        user = %config.user,
        bind_addr = %config.bind_addr,
        mapping_count = config.mappings.len(),
        daemon = config.daemon,
        insecure_accept_host_key = config.insecure_accept_host_key,
        "starting SSH forward session"
    );

    let listeners = bind_listeners(&config).await?;
    let auth = build_auth(&config.auth);
    let (session_tx, _session_rx) = watch::channel::<Option<SharedSession>>(None);

    for binding in listeners {
        tokio::spawn(run_listener(binding, session_tx.subscribe()));
    }

    supervise_sessions(&config, &auth, tunnel_id.as_deref(), session_tx).await
}

async fn bind_listeners(config: &ForwardConfig) -> anyhow::Result<Vec<ListenerBinding>> {
    let mut listeners = Vec::with_capacity(config.mappings.len());

    for mapping in &config.mappings {
        let local_addr = mapping.local_addr(config.bind_addr);
        let listener = TcpListener::bind(local_addr).await.map_err(|error| {
            anyhow!(
                "failed to bind local forward listener on {}: {}",
                local_addr,
                error
            )
        })?;

        info!(
            local = %local_addr,
            target = %mapping.target_addr(),
            "local forward listener bound"
        );

        listeners.push(ListenerBinding {
            mapping: mapping.clone(),
            listener,
        });
    }

    Ok(listeners)
}

async fn run_listener(
    binding: ListenerBinding,
    session_rx: watch::Receiver<Option<SharedSession>>,
) {
    let ListenerBinding { mapping, listener } = binding;
    let session_rx = session_rx;

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let mapping = mapping.clone();
                let session = session_rx.borrow().clone();
                tokio::spawn(async move {
                    if let Err(error) =
                        handle_local_connection(mapping, stream, peer_addr, session).await
                    {
                        warn!(peer_addr = %peer_addr, error = %error, "forward connection terminated with error");
                    }
                });
            }
            Err(error) => {
                error!(error = %error, mapping = %mapping.encoded_spec(), "failed to accept local forward connection");
                sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

async fn handle_local_connection(
    mapping: ForwardMapping,
    local_stream: TcpStream,
    peer_addr: SocketAddr,
    session: Option<SharedSession>,
) -> anyhow::Result<()> {
    let Some(session) = session else {
        warn!(
            peer_addr = %peer_addr,
            mapping = %mapping.encoded_spec(),
            "dropping local forward connection because SSH session is unavailable"
        );
        return Ok(());
    };

    let originator_address = peer_addr.ip().to_string();
    let originator_port = peer_addr.port();
    let remote_stream = {
        let session = session.lock().await;
        if session.is_closed() {
            return Err(anyhow!("SSH session is unavailable"));
        }

        session
            .open_direct_tcpip(
                &mapping.target_host,
                mapping.target_port,
                &originator_address,
                originator_port,
            )
            .await?
    };

    forwarder::bridge(local_stream, remote_stream).await
}

async fn supervise_sessions(
    config: &ForwardConfig,
    auth: &RuntimeAuth,
    tunnel_id: Option<&str>,
    session_tx: watch::Sender<Option<SharedSession>>,
) -> anyhow::Result<()> {
    let mut backoff = ReconnectBackoff::new();
    let mut reconnect_attempt = 0u32;

    loop {
        match run_single_session(config, auth, tunnel_id, &session_tx).await {
            Ok(()) => return Ok(()),
            Err(SessionFailure::Terminal(error)) => return Err(error),
            Err(SessionFailure::Retryable { error, established }) => {
                if established {
                    backoff.reset();
                    reconnect_attempt = 0;
                }

                reconnect_attempt += 1;
                let delay = backoff.next_delay();
                let _ = session_tx.send(None);

                if established && let Some(id) = tunnel_id {
                    let _ = tunnel_registry::mark_reconnecting(id);
                }

                warn!(
                    tunnel_id = tunnel_id.unwrap_or("foreground"),
                    server = %config.server,
                    user = %config.user,
                    attempt = reconnect_attempt,
                    delay_secs = delay.as_secs(),
                    error = %error,
                    "SSH forward session ended; retrying tunnel after backoff"
                );

                sleep(delay).await;
            }
        }
    }
}

async fn run_single_session(
    config: &ForwardConfig,
    auth: &RuntimeAuth,
    tunnel_id: Option<&str>,
    session_tx: &watch::Sender<Option<SharedSession>>,
) -> Result<(), SessionFailure> {
    let session = SshForwardSession::connect(&config.server, &config.user, auth)
        .await
        .map_err(classify_connect_error)?;
    let session = Arc::new(Mutex::new(session));

    let _ = session_tx.send(Some(session.clone()));
    if let Some(id) = tunnel_id {
        let _ = tunnel_registry::mark_running(id);
    }

    loop {
        sleep(SESSION_POLL_INTERVAL).await;
        if session.lock().await.is_closed() {
            break;
        }
    }

    let _ = session_tx.send(None);
    Err(SessionFailure::Retryable {
        error: anyhow!("forward session terminated unexpectedly: SSH session ended"),
        established: true,
    })
}

fn classify_connect_error(error: ConnectError) -> SessionFailure {
    match error {
        ConnectError::Transport(error) => SessionFailure::Retryable {
            error,
            established: false,
        },
        ConnectError::InvalidPrivateKey(error) | ConnectError::AuthenticationRejected(error) => {
            SessionFailure::Terminal(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ForwardConfig, ForwardMapping, ForwardRemoteSpec};
    use crate::cli::ForwardArgs;

    #[test]
    fn parses_forward_remote_with_default_port() {
        let spec = ForwardRemoteSpec::parse("root@example.com").unwrap();

        assert_eq!(spec.user.as_deref(), Some("root"));
        assert_eq!(spec.server_host, "example.com");
        assert_eq!(spec.server_port, 22);
    }

    #[test]
    fn parses_forward_remote_with_explicit_port() {
        let spec = ForwardRemoteSpec::parse("example.com:2222").unwrap();

        assert_eq!(spec.user, None);
        assert_eq!(spec.server_host, "example.com");
        assert_eq!(spec.server_port, 2222);
    }

    #[test]
    fn parses_forward_local_spec() {
        let spec = ForwardMapping::parse("8080:web.internal:80").unwrap();

        assert_eq!(spec.local_port, 8080);
        assert_eq!(spec.target_host, "web.internal");
        assert_eq!(spec.target_port, 80);
    }

    #[test]
    fn forward_args_convert_to_config_with_multiple_mappings() {
        let config = ForwardConfig::try_from(ForwardArgs {
            local: vec![
                "8080:web.internal:80".to_string(),
                "5432:db.internal:5432".to_string(),
            ],
            remote: "root@example.com:2222".to_string(),
            user: None,
            bind: None,
            gateway: false,
            password: Some("secret".to_string()),
            key: None,
            daemon: false,
            log_file: None,
            insecure_accept_host_key: true,
        })
        .unwrap();

        assert_eq!(config.server, "example.com:2222");
        assert_eq!(config.user, "root");
        assert_eq!(config.bind_addr.to_string(), "127.0.0.1");
        assert_eq!(config.mappings.len(), 2);
    }

    #[test]
    fn rejects_duplicate_forward_binds() {
        let error = ForwardConfig::try_from(ForwardArgs {
            local: vec![
                "8080:web.internal:80".to_string(),
                "8080:db.internal:5432".to_string(),
            ],
            remote: "root@example.com".to_string(),
            user: None,
            bind: None,
            gateway: false,
            password: Some("secret".to_string()),
            key: None,
            daemon: false,
            log_file: None,
            insecure_accept_host_key: true,
        })
        .unwrap_err();

        assert!(error.to_string().contains("duplicate local forward bind"));
    }

    #[test]
    fn rejects_missing_forward_user() {
        let error = ForwardConfig::try_from(ForwardArgs {
            local: vec!["8080:web.internal:80".to_string()],
            remote: "example.com".to_string(),
            user: None,
            bind: None,
            gateway: false,
            password: Some("secret".to_string()),
            key: None,
            daemon: false,
            log_file: None,
            insecure_accept_host_key: true,
        })
        .unwrap_err();

        assert!(error.to_string().contains("SSH username must be provided"));
    }
}
