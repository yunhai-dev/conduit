use crate::expose::types::{AuthConfig, ExposeConfig};

#[derive(Debug, Clone)]
pub enum RuntimeAuth {
    Password(String),
    PrivateKeyFile(std::path::PathBuf),
}

pub fn build_auth(config: &ExposeConfig) -> RuntimeAuth {
    match &config.auth {
        AuthConfig::Password(password) => RuntimeAuth::Password(password.clone()),
        AuthConfig::PrivateKey(path) => RuntimeAuth::PrivateKeyFile(path.clone()),
    }
}
