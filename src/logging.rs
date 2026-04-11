use std::path::Path;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init(log_file: Option<&Path>) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Some(path) = log_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        fmt()
            .with_env_filter(env_filter)
            .with_writer(file)
            .with_target(false)
            .init();
    } else {
        fmt().with_env_filter(env_filter).with_target(false).init();
    }

    Ok(())
}
