mod app;
mod cli;
mod daemon;
mod error;
mod expose;
mod forward;
mod logging;
mod tunnel_registry;

use crate::app::{PreparedDaemonCommand, PreparedTunnelConfig};
use crate::cli::Cli;
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    let prepared_daemon = match app::prepare_daemon_command(&cli) {
        Ok(prepared) => prepared,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    };

    if let Some(prepared) = prepared_daemon {
        run_daemon(prepared);
        return;
    }

    if let Err(error) = logging::init(None) {
        eprintln!("failed to initialize logging: {error}");
        std::process::exit(1);
    }

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(error) = runtime.block_on(app::run(cli)) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run_daemon(mut prepared: PreparedDaemonCommand) {
    match &mut prepared.config {
        PreparedTunnelConfig::Reverse(config) => {
            config.log_file = Some(prepared.launch.log_file.clone());
        }
        PreparedTunnelConfig::Forward(config) => {
            config.log_file = Some(prepared.launch.log_file.clone());
        }
    }

    if let Err(error) = daemon::daemonize(&prepared.launch.log_file) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }

    if let Some(id) = prepared.close_before_launch.as_deref()
        && let Err(error) = tunnel_registry::close_tunnel(id)
    {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }

    if let Err(error) = logging::init(Some(prepared.launch.log_file.as_path())) {
        eprintln!("failed to initialize logging: {error}");
        std::process::exit(1);
    }

    let record_result = match &prepared.config {
        PreparedTunnelConfig::Reverse(config) => tunnel_registry::create_starting_record(
            &prepared.launch.tunnel_id,
            &prepared.command,
            config,
            std::process::id(),
        ),
        PreparedTunnelConfig::Forward(config) => tunnel_registry::create_starting_forward_record(
            &prepared.launch.tunnel_id,
            &prepared.command,
            config,
            std::process::id(),
        ),
    };

    if let Err(error) = record_result {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(error) = runtime.block_on(app::run_prepared_daemon(prepared)) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
