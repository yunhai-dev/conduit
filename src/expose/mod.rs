pub mod auth;
pub mod forwarder;
pub mod ssh_client;
pub mod types;

use crate::expose::auth::{RuntimeAuth, build_auth};
use crate::expose::ssh_client::{ConnectError, ForwardError, SshTunnel};
use crate::expose::types::ExposeConfig;
use crate::tunnel_registry;
use anyhow::anyhow;
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

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

pub async fn run(config: ExposeConfig, tunnel_id: Option<String>) -> anyhow::Result<()> {
    info!(
        server = %config.server,
        user = %config.user,
        local = %config.local,
        remote_host = %config.remote_host,
        remote_port = config.remote_port,
        auth = %config.auth,
        insecure_accept_host_key = config.insecure_accept_host_key,
        "starting SSH expose session"
    );

    let auth = build_auth(&config.auth);
    let mut backoff = ReconnectBackoff::new();
    let mut reconnect_attempt = 0u32;

    loop {
        match run_single_session(&config, &auth, tunnel_id.as_deref()).await {
            Ok(()) => return Ok(()),
            Err(SessionFailure::Terminal(error)) => return Err(error),
            Err(SessionFailure::Retryable { error, established }) => {
                if established {
                    backoff.reset();
                    reconnect_attempt = 0;
                }

                reconnect_attempt += 1;
                let delay = backoff.next_delay();

                if established && let Some(id) = tunnel_id.as_deref() {
                    let _ = tunnel_registry::mark_reconnecting(id);
                }

                warn!(
                    tunnel_id = tunnel_id.as_deref().unwrap_or("foreground"),
                    server = %config.server,
                    user = %config.user,
                    attempt = reconnect_attempt,
                    delay_secs = delay.as_secs(),
                    error = %error,
                    "SSH session ended; retrying tunnel after backoff"
                );

                sleep(delay).await;
            }
        }
    }
}

async fn run_single_session(
    config: &ExposeConfig,
    auth: &RuntimeAuth,
    tunnel_id: Option<&str>,
) -> Result<(), SessionFailure> {
    let mut tunnel = SshTunnel::connect(config, auth)
        .await
        .map_err(classify_connect_error)?;
    let bound_port = tunnel
        .request_remote_forward(&config.remote_host, config.remote_port)
        .await
        .map_err(classify_forward_error)?;

    info!(
        remote_host = %config.remote_host,
        requested_remote_port = config.remote_port,
        bound_port,
        local = %config.local,
        "remote forwarding established"
    );

    if let Some(id) = tunnel_id {
        let _ = tunnel_registry::mark_running(id);
    }

    while let Some(connection) = tunnel.next_forwarded_connection().await {
        info!(
            connected_address = %connection.connected_address,
            connected_port = connection.connected_port,
            originator_address = %connection.originator_address,
            originator_port = connection.originator_port,
            "accepted forwarded connection"
        );

        let local = config.local;
        tokio::spawn(async move {
            let originator_address = connection.originator_address.clone();
            let originator_port = connection.originator_port;

            match TcpStream::connect(local).await {
                Ok(local_stream) => {
                    if let Err(error) = forwarder::bridge(local_stream, connection.channel).await {
                        warn!(
                            local = %local,
                            originator_address = %originator_address,
                            originator_port,
                            error = %error,
                            "forwarded connection terminated with error"
                        );
                    }
                }
                Err(error) => {
                    error!(
                        local = %local,
                        originator_address = %originator_address,
                        originator_port,
                        error = %error,
                        "failed to connect to local target"
                    );
                }
            }
        });
    }

    Err(SessionFailure::Retryable {
        error: anyhow!(
            "expose loop terminated unexpectedly: SSH session ended while waiting for forwarded connections"
        ),
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

fn classify_forward_error(error: ForwardError) -> SessionFailure {
    match error {
        ForwardError::RequestDenied {
            remote_host,
            remote_port,
        } => SessionFailure::Terminal(anyhow!(
            "SSH server denied remote forward request on {}:{}",
            remote_host,
            remote_port
        )),
        ForwardError::Request {
            remote_host,
            remote_port,
            source,
        } => SessionFailure::Retryable {
            error: anyhow!(
                "failed to register remote forward on {}:{}: {}",
                remote_host,
                remote_port,
                source
            ),
            established: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use russh::Error as RusshError;

    #[test]
    fn reconnect_backoff_caps_and_resets() {
        let mut backoff = ReconnectBackoff::new();

        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
        assert_eq!(backoff.next_delay(), Duration::from_secs(2));
        assert_eq!(backoff.next_delay(), Duration::from_secs(4));
        assert_eq!(backoff.next_delay(), Duration::from_secs(8));
        assert_eq!(backoff.next_delay(), Duration::from_secs(16));
        assert_eq!(backoff.next_delay(), Duration::from_secs(30));
        assert_eq!(backoff.next_delay(), Duration::from_secs(30));

        backoff.reset();
        assert_eq!(backoff.next_delay(), Duration::from_secs(1));
    }

    #[test]
    fn transport_connect_errors_retry() {
        let failure = classify_connect_error(ConnectError::Transport(anyhow!("network down")));
        assert!(matches!(
            failure,
            SessionFailure::Retryable {
                established: false,
                ..
            }
        ));
    }

    #[test]
    fn auth_rejection_is_terminal() {
        let failure = classify_connect_error(ConnectError::AuthenticationRejected(anyhow!(
            "bad password"
        )));
        assert!(matches!(failure, SessionFailure::Terminal(_)));
    }

    #[test]
    fn request_denied_is_terminal() {
        let failure = classify_forward_error(ForwardError::RequestDenied {
            remote_host: "0.0.0.0".to_string(),
            remote_port: 8080,
        });
        assert!(matches!(failure, SessionFailure::Terminal(_)));
    }

    #[test]
    fn forward_transport_errors_retry() {
        let failure = classify_forward_error(ForwardError::Request {
            remote_host: "0.0.0.0".to_string(),
            remote_port: 8080,
            source: RusshError::Disconnect,
        });
        assert!(matches!(
            failure,
            SessionFailure::Retryable {
                established: false,
                ..
            }
        ));
    }
}
