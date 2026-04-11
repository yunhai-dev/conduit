use crate::error::ConduitError;
use crate::expose::types::ExposeConfig;
use anyhow::{Context, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const STATE_DIR_NAME: &str = ".conduit";
const TUNNELS_DIR_NAME: &str = "tunnels";
const LOGS_DIR_NAME: &str = "logs";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelStatus {
    Starting,
    Running,
    Exited,
    Failed,
    Stale,
}

impl TunnelStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Exited => "exited",
            Self::Failed => "failed",
            Self::Stale => "stale",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "exited" => Some(Self::Exited),
            "failed" => Some(Self::Failed),
            "stale" => Some(Self::Stale),
            _ => None,
        }
    }

    fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}

#[derive(Debug, Clone)]
pub struct TunnelRecord {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub server: String,
    pub user: String,
    pub local: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub daemon: bool,
    pub log_file: PathBuf,
    pub status: TunnelStatus,
}

impl TunnelRecord {
    fn to_wire(&self) -> String {
        [
            ("id", self.id.as_str()),
            ("command", self.command.as_str()),
            ("pid", &self.pid.to_string()),
            ("created_at", &self.created_at.to_string()),
            ("updated_at", &self.updated_at.to_string()),
            ("server", self.server.as_str()),
            ("user", self.user.as_str()),
            ("local", self.local.as_str()),
            ("remote_host", self.remote_host.as_str()),
            ("remote_port", &self.remote_port.to_string()),
            ("daemon", if self.daemon { "true" } else { "false" }),
            ("log_file", self.log_file.to_string_lossy().as_ref()),
            ("status", self.status.as_str()),
        ]
        .into_iter()
        .map(|(key, value)| format!("{key}={}\n", escape(value)))
        .collect()
    }

    fn from_wire(input: &str) -> anyhow::Result<Self> {
        let mut id = None;
        let mut command = None;
        let mut pid = None;
        let mut created_at = None;
        let mut updated_at = None;
        let mut server = None;
        let mut user = None;
        let mut local = None;
        let mut remote_host = None;
        let mut remote_port = None;
        let mut daemon = None;
        let mut log_file = None;
        let mut status = None;

        for line in input.lines().filter(|line| !line.trim().is_empty()) {
            let (key, value) = line
                .split_once('=')
                .context("invalid tunnel registry line")?;
            let value = unescape(value);

            match key {
                "id" => id = Some(value),
                "command" => command = Some(value),
                "pid" => pid = Some(value.parse::<u32>().context("invalid tunnel pid")?),
                "created_at" => {
                    created_at = Some(value.parse::<u64>().context("invalid created_at")?)
                }
                "updated_at" => {
                    updated_at = Some(value.parse::<u64>().context("invalid updated_at")?)
                }
                "server" => server = Some(value),
                "user" => user = Some(value),
                "local" => local = Some(value),
                "remote_host" => remote_host = Some(value),
                "remote_port" => {
                    remote_port = Some(value.parse::<u16>().context("invalid remote_port")?)
                }
                "daemon" => daemon = Some(matches!(value.as_str(), "true")),
                "log_file" => log_file = Some(PathBuf::from(value)),
                "status" => {
                    status = TunnelStatus::parse(&value);
                }
                _ => {}
            }
        }

        Ok(Self {
            id: id.context("missing tunnel id")?,
            command: command.context("missing tunnel command")?,
            pid: pid.context("missing tunnel pid")?,
            created_at: created_at.context("missing created_at")?,
            updated_at: updated_at.context("missing updated_at")?,
            server: server.context("missing tunnel server")?,
            user: user.context("missing tunnel user")?,
            local: local.context("missing tunnel local")?,
            remote_host: remote_host.context("missing tunnel remote_host")?,
            remote_port: remote_port.context("missing tunnel remote_port")?,
            daemon: daemon.context("missing tunnel daemon flag")?,
            log_file: log_file.context("missing tunnel log_file")?,
            status: status.context("missing tunnel status")?,
        })
    }
}

pub fn generate_tunnel_id() -> String {
    format!("tunnel-{}-{}", now_millis(), std::process::id())
}

pub fn default_log_path(id: &str) -> anyhow::Result<PathBuf> {
    let logs_dir = logs_dir()?;
    fs::create_dir_all(&logs_dir).context("failed to create conduit logs directory")?;
    Ok(logs_dir.join(format!("{id}.log")))
}

pub fn normalize_log_path(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to resolve current directory")?
            .join(path)
    };

    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log file directory {}", parent.display()))?;
    }

    Ok(absolute)
}

pub fn create_starting_record(
    id: &str,
    command: &str,
    config: &ExposeConfig,
    pid: u32,
) -> anyhow::Result<TunnelRecord> {
    let now = now_secs();
    let log_file = config
        .log_file
        .clone()
        .context("managed daemon tunnel requires a log file")?;
    let record = TunnelRecord {
        id: id.to_string(),
        command: command.to_string(),
        pid,
        created_at: now,
        updated_at: now,
        server: config.server.clone(),
        user: config.user.clone(),
        local: config.local.to_string(),
        remote_host: config.remote_host.clone(),
        remote_port: config.remote_port,
        daemon: config.daemon,
        log_file,
        status: TunnelStatus::Starting,
    };

    save_record(&record)?;
    Ok(record)
}

pub fn mark_running(id: &str) -> anyhow::Result<()> {
    update_status(id, TunnelStatus::Running)
}

pub fn mark_failed(id: &str) -> anyhow::Result<()> {
    update_status(id, TunnelStatus::Failed)
}

pub fn mark_exited(id: &str) -> anyhow::Result<()> {
    update_status(id, TunnelStatus::Exited)
}

pub fn list_records(include_all: bool) -> anyhow::Result<Vec<TunnelRecord>> {
    let tunnels_dir = tunnels_dir()?;
    if !tunnels_dir.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    for entry in fs::read_dir(&tunnels_dir).context("failed to read tunnel registry")? {
        let entry = entry.context("failed to read tunnel registry entry")?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("meta") {
            continue;
        }

        let mut record = read_record(&path)?;
        reconcile_record(&mut record)?;
        if include_all || record.status.is_active() {
            records.push(record);
        }
    }

    records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(records)
}

pub fn get_record(id: &str) -> anyhow::Result<TunnelRecord> {
    let path = record_path(id)?;
    if !path.exists() {
        return Err(ConduitError::UnknownTunnel(id.to_string()).into());
    }

    let mut record = read_record(&path)?;
    reconcile_record(&mut record)?;
    Ok(record)
}

pub fn close_tunnel(id: &str) -> anyhow::Result<TunnelRecord> {
    let mut record = get_record(id)?;

    if is_process_alive(record.pid) {
        terminate_process(record.pid)?;
        record.status = TunnelStatus::Exited;
    } else {
        record.status = TunnelStatus::Stale;
    }

    record.updated_at = now_secs();
    save_record(&record)?;
    Ok(record)
}

pub fn stop_all_tunnels(include_all: bool) -> anyhow::Result<Vec<TunnelRecord>> {
    let records = list_records(true)?;
    let mut stopped = Vec::new();

    for mut record in records {
        if !include_all && !record.status.is_active() {
            continue;
        }

        if is_process_alive(record.pid) {
            terminate_process(record.pid)?;
            record.status = TunnelStatus::Exited;
        } else {
            record.status = TunnelStatus::Stale;
        }

        record.updated_at = now_secs();
        save_record(&record)?;
        stopped.push(record);
    }

    Ok(stopped)
}

pub fn delete_tunnel(id: &str, force: bool) -> anyhow::Result<TunnelRecord> {
    let mut record = get_record(id)?;
    let is_alive = is_process_alive(record.pid);

    if is_alive && !force {
        return Err(ConduitError::ActiveTunnelDeleteRequiresForce(record.id).into());
    }

    if is_alive {
        terminate_process(record.pid)?;
        record.status = TunnelStatus::Exited;
        record.updated_at = now_secs();
    }

    let path = record_path(&record.id)?;
    fs::remove_file(&path)
        .with_context(|| format!("failed to delete tunnel record {}", path.display()))?;

    if record.log_file.exists() {
        fs::remove_file(&record.log_file).with_context(|| {
            format!(
                "failed to delete tunnel log file {}",
                record.log_file.display()
            )
        })?;
    }

    Ok(record)
}

fn update_status(id: &str, status: TunnelStatus) -> anyhow::Result<()> {
    let path = record_path(id)?;
    if !path.exists() {
        return Ok(());
    }

    let mut record = read_record(&path)?;
    record.status = status;
    record.updated_at = now_secs();
    save_record(&record)
}

fn reconcile_record(record: &mut TunnelRecord) -> anyhow::Result<()> {
    if record.status.is_active() && !is_process_alive(record.pid) {
        record.status = TunnelStatus::Stale;
        record.updated_at = now_secs();
        save_record(record)?;
    }

    Ok(())
}

fn save_record(record: &TunnelRecord) -> anyhow::Result<()> {
    let path = record_path(&record.id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create tunnel registry directory")?;
    }

    fs::write(&path, record.to_wire())
        .with_context(|| format!("failed to write tunnel record {}", path.display()))
}

fn read_record(path: &Path) -> anyhow::Result<TunnelRecord> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read tunnel record {}", path.display()))?;
    TunnelRecord::from_wire(&content)
}

fn record_path(id: &str) -> anyhow::Result<PathBuf> {
    Ok(tunnels_dir()?.join(format!("{id}.meta")))
}

fn state_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(STATE_DIR_NAME))
}

fn tunnels_dir() -> anyhow::Result<PathBuf> {
    Ok(state_dir()?.join(TUNNELS_DIR_NAME))
}

fn logs_dir() -> anyhow::Result<PathBuf> {
    Ok(state_dir()?.join(LOGS_DIR_NAME))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some(next) => result.push(next),
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

#[cfg(unix)]
fn terminate_process(pid: u32) -> anyhow::Result<()> {
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .context("failed to invoke kill")?;

    if !status.success() {
        bail!("failed to terminate tunnel process {pid}")
    }

    Ok(())
}

#[cfg(not(unix))]
fn terminate_process(_pid: u32) -> anyhow::Result<()> {
    bail!("managed tunnel close is only supported on Unix")
}

#[cfg(test)]
mod tests {
    use super::{TunnelRecord, TunnelStatus};
    use std::path::PathBuf;

    #[test]
    fn record_round_trips_through_wire_format() {
        let record = TunnelRecord {
            id: "tunnel-1".to_string(),
            command: "connect".to_string(),
            pid: 1234,
            created_at: 1,
            updated_at: 2,
            server: "example.com:22".to_string(),
            user: "root".to_string(),
            local: "127.0.0.1:3000".to_string(),
            remote_host: "0.0.0.0".to_string(),
            remote_port: 8080,
            daemon: true,
            log_file: PathBuf::from("/tmp/conduit.log"),
            status: TunnelStatus::Running,
        };

        let parsed = TunnelRecord::from_wire(&record.to_wire()).unwrap();

        assert_eq!(parsed.id, record.id);
        assert_eq!(parsed.command, record.command);
        assert_eq!(parsed.pid, record.pid);
        assert_eq!(parsed.server, record.server);
        assert_eq!(parsed.user, record.user);
        assert_eq!(parsed.local, record.local);
        assert_eq!(parsed.remote_host, record.remote_host);
        assert_eq!(parsed.remote_port, record.remote_port);
        assert_eq!(parsed.daemon, record.daemon);
        assert_eq!(parsed.log_file, record.log_file);
        assert_eq!(parsed.status, record.status);
    }
}
