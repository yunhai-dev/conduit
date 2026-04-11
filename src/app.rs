use crate::cli::{
    Cli, CloseArgs, Commands, ConnectArgs, DeleteArgs, ExposeArgs, ListArgs, LogsArgs, RestartArgs,
    StatusArgs, StopArgs, TunnelArgs, TunnelCommands,
};
use crate::error::ConduitError;
use crate::expose;
use crate::tunnel_registry;
use anyhow::Context;
use std::io::{self, BufRead, Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct PreparedDaemonLaunch {
    pub tunnel_id: String,
    pub log_file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PreparedDaemonCommand {
    pub command: String,
    pub config: expose::types::ExposeConfig,
    pub launch: PreparedDaemonLaunch,
    pub close_before_launch: Option<String>,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Connect(args) => run_connect(args).await,
        Commands::Expose(args) => run_expose(args).await,
        Commands::Tunnel(args) => run_tunnel(args).await,
        Commands::List(args) => run_list(args),
        Commands::Status(args) => run_status(args),
        Commands::Close(args) => run_close(args),
        Commands::Stop(args) => run_stop(args),
        Commands::Restart(args) => run_restart(args).await,
        Commands::Delete(args) => run_delete(args),
        Commands::Logs(args) => run_logs(args),
    }
}

pub async fn run_prepared_daemon(prepared: PreparedDaemonCommand) -> anyhow::Result<()> {
    run_tunnel_command(&prepared.command, prepared.config, Some(prepared.launch)).await
}

async fn run_connect(args: ConnectArgs) -> anyhow::Result<()> {
    let config = expose::types::ExposeConfig::try_from(args)?;
    run_tunnel_command("connect", config, None).await
}

async fn run_expose(args: ExposeArgs) -> anyhow::Result<()> {
    let config = expose::types::ExposeConfig::try_from(args)?;
    run_tunnel_command("expose", config, None).await
}

async fn run_tunnel(args: TunnelArgs) -> anyhow::Result<()> {
    match args.command {
        TunnelCommands::Create(args) => run_connect(args).await,
        TunnelCommands::List(args) => run_list(args),
        TunnelCommands::Show(args) => run_status(args),
        TunnelCommands::Close(args) => run_close(args),
        TunnelCommands::Stop(args) => run_stop(args),
        TunnelCommands::Restart(args) => run_restart(args).await,
        TunnelCommands::Delete(args) => run_delete(args),
        TunnelCommands::Logs(args) => run_logs(args),
    }
}

fn run_list(args: ListArgs) -> anyhow::Result<()> {
    let records = tunnel_registry::list_records(args.all)?;

    if records.is_empty() {
        println!("no managed tunnels");
        return Ok(());
    }

    println!("ID\tSTATUS\tPID\tREMOTE\tLOCAL");
    for record in records {
        println!(
            "{}\t{}\t{}\t{}:{}\t{}",
            record.id,
            record.status.as_str(),
            record.pid,
            record.remote_host,
            record.remote_port,
            record.local
        );
    }

    Ok(())
}

fn run_status(args: StatusArgs) -> anyhow::Result<()> {
    let record = tunnel_registry::get_record(&args.id)?;

    println!("id: {}", record.id);
    println!("command: {}", record.command);
    println!("status: {}", record.status.as_str());
    println!("pid: {}", record.pid);
    println!("server: {}", record.server);
    println!("user: {}", record.user);
    println!("remote: {}:{}", record.remote_host, record.remote_port);
    println!("local: {}", record.local);
    println!("log_file: {}", record.log_file.display());
    println!("created_at: {}", record.created_at);
    println!("updated_at: {}", record.updated_at);

    Ok(())
}

fn run_close(args: CloseArgs) -> anyhow::Result<()> {
    let record = tunnel_registry::close_tunnel(&args.id)?;
    println!(
        "closed tunnel {} (status: {})",
        record.id,
        record.status.as_str()
    );
    Ok(())
}

async fn run_restart(args: RestartArgs) -> anyhow::Result<()> {
    let record = tunnel_registry::get_record(&args.id)?;
    let prepared = prepare_restart(&record, args)?;
    run_prepared_daemon(prepared).await
}

fn run_delete(args: DeleteArgs) -> anyhow::Result<()> {
    let record = tunnel_registry::delete_tunnel(&args.id, args.force)?;
    println!("deleted tunnel {}", record.id);
    Ok(())
}

fn run_stop(args: StopArgs) -> anyhow::Result<()> {
    let records = tunnel_registry::stop_all_tunnels(args.all)?;
    if records.is_empty() {
        println!("no managed tunnels to stop");
        return Ok(());
    }

    println!("stopped {} tunnel(s)", records.len());
    for record in records {
        println!("{}\t{}", record.id, record.status.as_str());
    }

    Ok(())
}

fn run_logs(args: LogsArgs) -> anyhow::Result<()> {
    let record = tunnel_registry::get_record(&args.id)?;
    if !record.log_file.exists() {
        return Err(ConduitError::MissingTunnelLogFile {
            id: record.id,
            path: record.log_file,
        }
        .into());
    }

    let file = std::fs::File::open(&record.log_file)?;
    let reader = io::BufReader::new(file);
    let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
    let start = lines.len().saturating_sub(args.tail);
    for line in &lines[start..] {
        println!("{line}");
    }

    if args.follow {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .open(&record.log_file)?;
        file.seek(SeekFrom::End(0))?;
        loop {
            let mut reader = io::BufReader::new(&file);
            let mut line = String::new();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            print!("{line}");
            file.stream_position()?;
        }
    }

    Ok(())
}

pub fn prepare_daemon_command(cli: &Cli) -> anyhow::Result<Option<PreparedDaemonCommand>> {
    match &cli.command {
        Commands::Connect(args) if args.daemon => {
            let config = expose::types::ExposeConfig::try_from(args.clone())?;
            let launch = prepare_daemon_launch(&config, None)?;
            Ok(Some(PreparedDaemonCommand {
                command: "connect".to_string(),
                config,
                launch,
                close_before_launch: None,
            }))
        }
        Commands::Expose(args) if args.daemon => {
            let config = expose::types::ExposeConfig::try_from(args.clone())?;
            let launch = prepare_daemon_launch(&config, None)?;
            Ok(Some(PreparedDaemonCommand {
                command: "expose".to_string(),
                config,
                launch,
                close_before_launch: None,
            }))
        }
        Commands::Restart(args) => {
            let record = tunnel_registry::get_record(&args.id)?;
            Ok(Some(prepare_restart(&record, args.clone())?))
        }
        Commands::Tunnel(args) => match &args.command {
            TunnelCommands::Create(args) if args.daemon => {
                let config = expose::types::ExposeConfig::try_from(args.clone())?;
                let launch = prepare_daemon_launch(&config, None)?;
                Ok(Some(PreparedDaemonCommand {
                    command: "connect".to_string(),
                    config,
                    launch,
                    close_before_launch: None,
                }))
            }
            TunnelCommands::Restart(args) => {
                let record = tunnel_registry::get_record(&args.id)?;
                Ok(Some(prepare_restart(&record, args.clone())?))
            }
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

fn prepare_daemon_launch(
    config: &expose::types::ExposeConfig,
    tunnel_id: Option<String>,
) -> anyhow::Result<PreparedDaemonLaunch> {
    let tunnel_id = tunnel_id.unwrap_or_else(tunnel_registry::generate_tunnel_id);
    let log_file = match config.log_file.as_deref() {
        Some(path) => tunnel_registry::normalize_log_path(path)?,
        None => tunnel_registry::default_log_path(&tunnel_id)?,
    };

    Ok(PreparedDaemonLaunch {
        tunnel_id,
        log_file,
    })
}

async fn run_tunnel_command(
    command: &str,
    mut config: expose::types::ExposeConfig,
    prepared_daemon: Option<PreparedDaemonLaunch>,
) -> anyhow::Result<()> {
    let tunnel_id = if config.daemon {
        let prepared = prepared_daemon.context("missing prepared daemon launch")?;
        config.log_file = Some(prepared.log_file.clone());
        Some(prepared.tunnel_id)
    } else {
        None
    };

    info!(
        command,
        tunnel_id = tunnel_id.as_deref().unwrap_or("foreground"),
        server = %config.server,
        user = %config.user,
        local = %config.local,
        remote_host = %config.remote_host,
        remote_port = config.remote_port,
        daemon = config.daemon,
        "starting tunnel command"
    );

    match expose::run(config.clone(), tunnel_id.clone()).await {
        Ok(()) => {
            if let Some(id) = tunnel_id.as_deref() {
                let _ = tunnel_registry::mark_exited(id);
            }
            Ok(())
        }
        Err(error) => {
            if let Some(id) = tunnel_id.as_deref() {
                let _ = tunnel_registry::mark_failed(id);
            }
            Err(error)
        }
    }
}

fn prepare_restart(
    record: &tunnel_registry::TunnelRecord,
    args: RestartArgs,
) -> anyhow::Result<PreparedDaemonCommand> {
    let config = build_restart_config(record, args)?;

    Ok(PreparedDaemonCommand {
        command: record.command.clone(),
        config,
        launch: PreparedDaemonLaunch {
            tunnel_id: record.id.clone(),
            log_file: record.log_file.clone(),
        },
        close_before_launch: Some(record.id.clone()),
    })
}

fn build_restart_config(
    record: &tunnel_registry::TunnelRecord,
    args: RestartArgs,
) -> anyhow::Result<expose::types::ExposeConfig> {
    if !record.daemon {
        return Err(ConduitError::UnsupportedTunnelRestart {
            id: record.id.clone(),
            reason: "only daemon-managed tunnels can be restarted".to_string(),
        }
        .into());
    }

    let auth = match (args.password, args.key) {
        (Some(password), None) => expose::types::AuthConfig::Password(password),
        (None, Some(path)) => {
            if !path.exists() {
                return Err(ConduitError::MissingPrivateKey(path).into());
            }
            expose::types::AuthConfig::PrivateKey(path)
        }
        _ => {
            return Err(ConduitError::UnsupportedTunnelRestart {
                id: record.id.clone(),
                reason: "restart requires exactly one of --password or --key".to_string(),
            }
            .into());
        }
    };

    if !args.insecure_accept_host_key {
        return Err(ConduitError::MissingHostKeyPolicy.into());
    }

    let local =
        record
            .local
            .parse::<SocketAddr>()
            .map_err(|_| ConduitError::UnsupportedTunnelRestart {
                id: record.id.clone(),
                reason: format!("stored local target is invalid: {}", record.local),
            })?;

    Ok(expose::types::ExposeConfig {
        server: record.server.clone(),
        user: record.user.clone(),
        local,
        remote_host: record.remote_host.clone(),
        remote_port: record.remote_port,
        auth,
        daemon: true,
        log_file: Some(record.log_file.clone()),
        insecure_accept_host_key: args.insecure_accept_host_key,
    })
}
