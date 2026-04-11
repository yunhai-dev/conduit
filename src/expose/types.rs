use crate::cli::{ConnectArgs, ExposeArgs};
use crate::error::ConduitError;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ExposeConfig {
    pub server: String,
    pub user: String,
    pub local: SocketAddr,
    pub remote_host: String,
    pub remote_port: u16,
    pub auth: AuthConfig,
    pub daemon: bool,
    pub log_file: Option<PathBuf>,
    pub insecure_accept_host_key: bool,
}

#[derive(Debug, Clone)]
pub enum AuthConfig {
    Password(String),
    PrivateKey(PathBuf),
}

impl Display for AuthConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password(_) => write!(f, "password"),
            Self::PrivateKey(path) => write!(f, "private-key({})", path.display()),
        }
    }
}

pub fn parse_server_addr(server: &str) -> Result<(), ConduitError> {
    if server.parse::<std::net::SocketAddr>().is_ok() {
        return Ok(());
    }

    let (host, port) = if let Some(server) = server.strip_prefix('[') {
        let (host, port) = server
            .split_once(":")
            .ok_or_else(|| ConduitError::InvalidServerAddress(server.to_string()))?;
        (host, port)
    } else {
        let (host, port) = server
            .rsplit_once(':')
            .ok_or_else(|| ConduitError::InvalidServerAddress(server.to_string()))?;

        if host.contains(':') {
            return Err(ConduitError::InvalidServerAddress(server.to_string()));
        }

        (host, port)
    };

    if host.trim().is_empty() || host.contains(char::is_whitespace) {
        return Err(ConduitError::InvalidServerAddress(server.to_string()));
    }

    port.parse::<u16>()
        .map_err(|_| ConduitError::InvalidServerAddress(server.to_string()))?;

    Ok(())
}

pub fn validate_remote_host(remote_host: &str) -> Result<(), ConduitError> {
    if remote_host.trim().is_empty() || remote_host.contains(char::is_whitespace) {
        return Err(ConduitError::InvalidRemoteHost(remote_host.to_string()));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteSpec {
    user: Option<String>,
    server_host: String,
    remote_port: u16,
    local_host: String,
    local_port: u16,
}

impl RemoteSpec {
    fn parse(input: &str) -> Result<Self, ConduitError> {
        let (user, remainder) = match input.split_once('@') {
            Some((user, remainder)) if !user.is_empty() && !remainder.is_empty() => {
                (Some(user.to_string()), remainder)
            }
            Some(_) => return Err(ConduitError::InvalidRemoteSpec(input.to_string())),
            None => (None, input),
        };

        let parts = remainder.split(':').collect::<Vec<_>>();
        let (server_host, remote_port, local_host, local_port) = match parts.as_slice() {
            [server_host, remote_port, local_port] => {
                (*server_host, *remote_port, "127.0.0.1", *local_port)
            }
            [server_host, remote_port, local_host, local_port] => {
                (*server_host, *remote_port, *local_host, *local_port)
            }
            _ => return Err(ConduitError::InvalidRemoteSpec(input.to_string())),
        };

        if server_host.trim().is_empty() || local_host.trim().is_empty() {
            return Err(ConduitError::InvalidRemoteSpec(input.to_string()));
        }

        let remote_port = remote_port
            .parse::<u16>()
            .map_err(|_| ConduitError::InvalidRemoteSpec(input.to_string()))?;
        let local_port = local_port
            .parse::<u16>()
            .map_err(|_| ConduitError::InvalidRemoteSpec(input.to_string()))?;

        Ok(Self {
            user,
            server_host: server_host.to_string(),
            remote_port,
            local_host: local_host.to_string(),
            local_port,
        })
    }

    fn local_addr(&self) -> Result<SocketAddr, ConduitError> {
        format!("{}:{}", self.local_host, self.local_port)
            .parse::<SocketAddr>()
            .map_err(|_| ConduitError::InvalidRemoteSpec(self.render()))
    }

    fn render(&self) -> String {
        let mut spec = String::new();

        if let Some(user) = &self.user {
            spec.push_str(user);
            spec.push('@');
        }

        spec.push_str(&self.server_host);
        spec.push(':');
        spec.push_str(&self.remote_port.to_string());
        spec.push(':');
        spec.push_str(&self.local_host);
        spec.push(':');
        spec.push_str(&self.local_port.to_string());

        spec
    }
}

impl TryFrom<ConnectArgs> for ExposeConfig {
    type Error = anyhow::Error;

    fn try_from(args: ConnectArgs) -> Result<Self, Self::Error> {
        let remote = RemoteSpec::parse(&args.remote)?;
        let auth = match (args.password, args.key) {
            (Some(password), None) => AuthConfig::Password(password),
            (None, Some(path)) => {
                if !path.exists() {
                    return Err(ConduitError::MissingPrivateKey(path).into());
                }
                AuthConfig::PrivateKey(path)
            }
            _ => return Err(ConduitError::InvalidAuthConfig.into()),
        };

        let user = match (remote.user.as_ref(), args.user) {
            (Some(remote_user), Some(cli_user)) if remote_user != &cli_user => {
                return Err(ConduitError::ConflictingConnectUser {
                    remote_user: remote_user.clone(),
                    cli_user,
                }
                .into());
            }
            (Some(remote_user), Some(_)) => remote_user.clone(),
            (Some(remote_user), None) => remote_user.clone(),
            (None, Some(cli_user)) => cli_user,
            (None, None) => return Err(ConduitError::MissingConnectUser.into()),
        };

        let server = format!("{}:{}", remote.server_host, args.port);
        let local = remote.local_addr()?;

        parse_server_addr(&server)?;
        validate_remote_host("0.0.0.0")?;

        if !args.insecure_accept_host_key {
            return Err(ConduitError::MissingHostKeyPolicy.into());
        }

        Ok(Self {
            server,
            user,
            local,
            remote_host: "0.0.0.0".to_string(),
            remote_port: remote.remote_port,
            auth,
            daemon: args.daemon,
            log_file: args.log_file,
            insecure_accept_host_key: args.insecure_accept_host_key,
        })
    }
}

impl TryFrom<ExposeArgs> for ExposeConfig {
    type Error = anyhow::Error;

    fn try_from(args: ExposeArgs) -> Result<Self, Self::Error> {
        let auth = match (args.password, args.private_key) {
            (Some(password), None) => AuthConfig::Password(password),
            (None, Some(path)) => {
                if !path.exists() {
                    return Err(ConduitError::MissingPrivateKey(path).into());
                }
                AuthConfig::PrivateKey(path)
            }
            _ => return Err(ConduitError::InvalidAuthConfig.into()),
        };

        let local = args
            .local
            .parse::<SocketAddr>()
            .map_err(|_| ConduitError::InvalidLocalAddress(args.local.clone()))?;

        parse_server_addr(&args.server)?;
        validate_remote_host(&args.remote_host)?;

        if !args.insecure_accept_host_key {
            return Err(ConduitError::MissingHostKeyPolicy.into());
        }

        Ok(Self {
            server: args.server,
            user: args.user,
            local,
            remote_host: args.remote_host,
            remote_port: args.remote_port,
            auth,
            daemon: args.daemon,
            log_file: args.log_file,
            insecure_accept_host_key: args.insecure_accept_host_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectArgs, ExposeConfig, RemoteSpec, parse_server_addr, validate_remote_host};

    #[test]
    fn accepts_ipv4_server_addr() {
        assert!(parse_server_addr("192.168.0.109:22").is_ok());
    }

    #[test]
    fn accepts_hostname_server_addr() {
        assert!(parse_server_addr("example.com:22").is_ok());
    }

    #[test]
    fn accepts_bracketed_ipv6_server_addr() {
        assert!(parse_server_addr("[::1]:22").is_ok());
    }

    #[test]
    fn rejects_server_addr_without_port() {
        assert!(parse_server_addr("192.168.0.109").is_err());
    }

    #[test]
    fn rejects_server_addr_with_invalid_port() {
        assert!(parse_server_addr("example.com:not-a-port").is_err());
    }

    #[test]
    fn rejects_blank_remote_host() {
        assert!(validate_remote_host("   ").is_err());
    }

    #[test]
    fn accepts_hostname_remote_host() {
        assert!(validate_remote_host("public.example.com").is_ok());
    }

    #[test]
    fn parses_short_remote_spec() {
        let spec = RemoteSpec::parse("root@example.com:8080:3000").unwrap();

        assert_eq!(spec.user.as_deref(), Some("root"));
        assert_eq!(spec.server_host, "example.com");
        assert_eq!(spec.remote_port, 8080);
        assert_eq!(spec.local_host, "127.0.0.1");
        assert_eq!(spec.local_port, 3000);
    }

    #[test]
    fn parses_full_remote_spec() {
        let spec = RemoteSpec::parse("example.com:8080:192.168.1.10:3000").unwrap();

        assert_eq!(spec.user, None);
        assert_eq!(spec.server_host, "example.com");
        assert_eq!(spec.remote_port, 8080);
        assert_eq!(spec.local_host, "192.168.1.10");
        assert_eq!(spec.local_port, 3000);
    }

    #[test]
    fn rejects_remote_spec_with_invalid_shape() {
        assert!(RemoteSpec::parse("example.com:8080").is_err());
    }

    #[test]
    fn rejects_remote_spec_with_invalid_remote_port() {
        assert!(RemoteSpec::parse("example.com:not-a-port:3000").is_err());
    }

    #[test]
    fn rejects_remote_spec_with_invalid_local_target() {
        let spec = RemoteSpec::parse("example.com:8080:localhost:3000").unwrap();
        assert!(spec.local_addr().is_err());
    }

    #[test]
    fn connect_args_convert_to_expose_config() {
        let config = ExposeConfig::try_from(ConnectArgs {
            remote: "root@example.com:8080:3000".to_string(),
            user: None,
            port: 22,
            password: Some("secret".to_string()),
            key: None,
            daemon: false,
            log_file: None,
            insecure_accept_host_key: true,
        })
        .unwrap();

        assert_eq!(config.server, "example.com:22");
        assert_eq!(config.user, "root");
        assert_eq!(config.local.to_string(), "127.0.0.1:3000");
        assert_eq!(config.remote_host, "0.0.0.0");
        assert_eq!(config.remote_port, 8080);
    }

    #[test]
    fn connect_args_require_user() {
        let error = ExposeConfig::try_from(ConnectArgs {
            remote: "example.com:8080:3000".to_string(),
            user: None,
            port: 22,
            password: Some("secret".to_string()),
            key: None,
            daemon: false,
            log_file: None,
            insecure_accept_host_key: true,
        })
        .unwrap_err();

        assert!(error.to_string().contains("SSH username must be provided"));
    }

    #[test]
    fn connect_args_reject_conflicting_users() {
        let error = ExposeConfig::try_from(ConnectArgs {
            remote: "root@example.com:8080:3000".to_string(),
            user: Some("admin".to_string()),
            port: 22,
            password: Some("secret".to_string()),
            key: None,
            daemon: false,
            log_file: None,
            insecure_accept_host_key: true,
        })
        .unwrap_err();

        assert!(error.to_string().contains("conflicts with --user"));
    }
}
