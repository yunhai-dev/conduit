use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConduitError {
    #[error("SSH authentication requires exactly one of --password or --private-key")]
    InvalidAuthConfig,

    #[error("--server must be a valid host:port or [ipv6]:port address: {0}")]
    InvalidServerAddress(String),

    #[error("--local must be a valid socket address: {0}")]
    InvalidLocalAddress(String),

    #[error("--remote-host must be a valid IP address or hostname: {0}")]
    InvalidRemoteHost(String),

    #[error(
        "--remote must be in the form [USER@]SERVER:REMOTE_PORT:LOCAL_PORT or [USER@]SERVER:REMOTE_PORT:LOCAL_HOST:LOCAL_PORT: {0}"
    )]
    InvalidRemoteSpec(String),

    #[error("SSH username must be provided either via --user or USER@ in --remote")]
    MissingConnectUser,

    #[error("SSH username in --remote conflicts with --user: {remote_user} != {cli_user}")]
    ConflictingConnectUser {
        remote_user: String,
        cli_user: String,
    },

    #[error("unknown tunnel id: {0}")]
    UnknownTunnel(String),

    #[error("log file does not exist for tunnel {id}: {path}")]
    MissingTunnelLogFile { id: String, path: PathBuf },

    #[error("tunnel {id} cannot be restarted: {reason}")]
    UnsupportedTunnelRestart { id: String, reason: String },

    #[error("tunnel {0} is still running; close it first or pass --force")]
    ActiveTunnelDeleteRequiresForce(String),

    #[error("private key file does not exist: {0}")]
    MissingPrivateKey(PathBuf),

    #[error("host key verification requires --insecure-accept-host-key in V1")]
    MissingHostKeyPolicy,

    #[cfg(not(unix))]
    #[error("daemon mode is only supported on Unix platforms")]
    UnsupportedDaemonMode,
}
