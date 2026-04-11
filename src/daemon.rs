#[cfg(not(unix))]
use crate::error::ConduitError;
use std::path::Path;

#[cfg(unix)]
pub fn daemonize(log_file: &Path) -> anyhow::Result<()> {
    use daemonize::Daemonize;
    use std::fs::OpenOptions;

    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    Daemonize::new().stdout(stdout).stderr(stderr).start()?;
    Ok(())
}

#[cfg(not(unix))]
pub fn daemonize(_log_file: &Path) -> anyhow::Result<()> {
    Err(ConduitError::UnsupportedDaemonMode.into())
}
