use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "conduit", version, about = "Pure Rust SSH tunnel CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Connect(ConnectArgs),
    Expose(ExposeArgs),
    Forward(ForwardArgs),
    Tunnel(TunnelArgs),
    List(ListArgs),
    #[command(alias = "show")]
    Status(StatusArgs),
    Close(CloseArgs),
    Stop(StopArgs),
    Restart(RestartArgs),
    Delete(DeleteArgs),
    Logs(LogsArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ConnectArgs {
    #[arg(
        long,
        short = 'r',
        value_name = "SPEC",
        help = "Remote mapping, format: [USER@]SERVER:REMOTE_PORT:LOCAL_PORT or [USER@]SERVER:REMOTE_PORT:LOCAL_HOST:LOCAL_PORT"
    )]
    pub remote: String,

    #[arg(
        long,
        help = "SSH username (overrides USER@ in --remote when matching)"
    )]
    pub user: Option<String>,

    #[arg(long, default_value_t = 22, help = "SSH server port")]
    pub port: u16,

    #[arg(long, help = "SSH password authentication")]
    pub password: Option<String>,

    #[arg(long = "key", value_name = "PATH", help = "SSH private key path")]
    pub key: Option<PathBuf>,

    #[arg(long, help = "Run in background")]
    pub daemon: bool,

    #[arg(long, value_name = "PATH", help = "Log file path for daemon mode")]
    pub log_file: Option<PathBuf>,

    #[arg(long, help = "Accept remote host key without verification")]
    pub insecure_accept_host_key: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ForwardArgs {
    #[arg(
        long = "local",
        short = 'L',
        value_name = "SPEC",
        required = true,
        help = "Local mapping, format: LOCAL_PORT:TARGET_HOST:TARGET_PORT"
    )]
    pub local: Vec<String>,

    #[arg(
        long = "remote",
        short = 'r',
        value_name = "SERVER",
        help = "SSH server, format: [USER@]SERVER[:PORT]"
    )]
    pub remote: String,

    #[arg(
        long,
        help = "SSH username (overrides USER@ in --remote when matching)"
    )]
    pub user: Option<String>,

    #[arg(
        long,
        value_name = "ADDR",
        help = "Local bind address (defaults to 127.0.0.1)"
    )]
    pub bind: Option<String>,

    #[arg(
        long,
        help = "Bind local listeners on 0.0.0.0",
        conflicts_with = "bind"
    )]
    pub gateway: bool,

    #[arg(long, help = "SSH password authentication")]
    pub password: Option<String>,

    #[arg(long = "key", value_name = "PATH", help = "SSH private key path")]
    pub key: Option<PathBuf>,

    #[arg(long, help = "Run in background")]
    pub daemon: bool,

    #[arg(long, value_name = "PATH", help = "Log file path for daemon mode")]
    pub log_file: Option<PathBuf>,

    #[arg(long, help = "Accept remote host key without verification")]
    pub insecure_accept_host_key: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TunnelArgs {
    #[command(subcommand)]
    pub command: TunnelCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TunnelCommands {
    Create(ConnectArgs),
    List(ListArgs),
    #[command(alias = "status")]
    Show(StatusArgs),
    Close(CloseArgs),
    Stop(StopArgs),
    Restart(RestartArgs),
    Delete(DeleteArgs),
    Logs(LogsArgs),
}

#[derive(Debug, Clone, Args, Default)]
pub struct ListArgs {
    #[arg(long, short = 'a', help = "Show all tunnels, including inactive ones")]
    pub all: bool,
}

#[derive(Debug, Clone, Args)]
pub struct StatusArgs {
    #[arg(value_name = "ID", help = "Tunnel id")]
    pub id: String,
}

#[derive(Debug, Clone, Args)]
pub struct CloseArgs {
    #[arg(value_name = "ID", help = "Tunnel id")]
    pub id: String,
}

#[derive(Debug, Clone, Args, Default)]
pub struct StopArgs {
    #[arg(long, help = "Stop all managed tunnels, including inactive ones")]
    pub all: bool,
}

#[derive(Debug, Clone, Args)]
pub struct RestartArgs {
    #[arg(value_name = "ID", help = "Tunnel id")]
    pub id: String,

    #[arg(long, help = "SSH password authentication")]
    pub password: Option<String>,

    #[arg(long = "key", value_name = "PATH", help = "SSH private key path")]
    pub key: Option<PathBuf>,

    #[arg(long, help = "Accept remote host key without verification")]
    pub insecure_accept_host_key: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DeleteArgs {
    #[arg(value_name = "ID", help = "Tunnel id")]
    pub id: String,

    #[arg(long, help = "Delete even if the tunnel is still running")]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct LogsArgs {
    #[arg(value_name = "ID", help = "Tunnel id")]
    pub id: String,

    #[arg(long, short = 'f', help = "Follow log output")]
    pub follow: bool,

    #[arg(long, default_value_t = 100, help = "Show last N lines")]
    pub tail: usize,
}

#[derive(Debug, Clone, Args)]
pub struct ExposeArgs {
    #[arg(long, help = "SSH server address, e.g. example.com:22")]
    pub server: String,

    #[arg(long, help = "SSH username")]
    pub user: String,

    #[arg(long, help = "Local TCP target, e.g. 127.0.0.1:3000")]
    pub local: String,

    #[arg(long, help = "Remote TCP port to expose")]
    pub remote_port: u16,

    #[arg(long, default_value = "0.0.0.0", help = "Remote bind host")]
    pub remote_host: String,

    #[arg(long, help = "SSH password authentication")]
    pub password: Option<String>,

    #[arg(long, value_name = "PATH", help = "SSH private key path")]
    pub private_key: Option<PathBuf>,

    #[arg(long, help = "Run in background")]
    pub daemon: bool,

    #[arg(long, value_name = "PATH", help = "Log file path for daemon mode")]
    pub log_file: Option<PathBuf>,

    #[arg(long, help = "Accept remote host key without verification")]
    pub insecure_accept_host_key: bool,
}
