use crate::expose::types::AuthConfig;

#[derive(Debug, Clone)]
pub enum RuntimeAuth {
    Password(String),
    PrivateKeyFile(std::path::PathBuf),
}

pub fn build_auth(auth: &AuthConfig) -> RuntimeAuth {
    match auth {
        AuthConfig::Password(password) => RuntimeAuth::Password(password.clone()),
        AuthConfig::PrivateKey(path) => RuntimeAuth::PrivateKeyFile(path.clone()),
    }
}
