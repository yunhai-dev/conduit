pub mod auth;
pub mod forwarder;
pub mod ssh_client;
pub mod types;

use crate::expose::auth::build_auth;
use crate::expose::ssh_client::SshTunnel;
use crate::expose::types::ExposeConfig;
use crate::tunnel_registry;
use anyhow::Context;
use tokio::net::TcpStream;
use tracing::{error, info, warn};

pub async fn run(config: ExposeConfig, tunnel_id: Option<String>) -> anyhow::Result<()> {
    info!(
        server = %config.server,
        user = %config.user,
        local = %config.local,
        remote_host = %config.remote_host,
        remote_port = config.remote_port,
        auth = %config.auth,
        insecure_accept_host_key = config.insecure_accept_host_key,
        "starting SSH expose session"
    );

    let auth = build_auth(&config);
    let mut tunnel = SshTunnel::connect(&config, &auth).await?;
    let bound_port = tunnel
        .request_remote_forward(&config.remote_host, config.remote_port)
        .await?;

    info!(
        remote_host = %config.remote_host,
        requested_remote_port = config.remote_port,
        bound_port,
        local = %config.local,
        "remote forwarding established"
    );

    if let Some(id) = tunnel_id.as_deref() {
        let _ = tunnel_registry::mark_running(id);
    }

    while let Some(connection) = tunnel.next_forwarded_connection().await {
        info!(
            connected_address = %connection.connected_address,
            connected_port = connection.connected_port,
            originator_address = %connection.originator_address,
            originator_port = connection.originator_port,
            "accepted forwarded connection"
        );

        let local = config.local;
        tokio::spawn(async move {
            let originator_address = connection.originator_address.clone();
            let originator_port = connection.originator_port;

            match TcpStream::connect(local).await {
                Ok(local_stream) => {
                    if let Err(error) = forwarder::bridge(local_stream, connection.channel).await {
                        warn!(
                            local = %local,
                            originator_address = %originator_address,
                            originator_port,
                            error = %error,
                            "forwarded connection terminated with error"
                        );
                    }
                }
                Err(error) => {
                    error!(
                        local = %local,
                        originator_address = %originator_address,
                        originator_port,
                        error = %error,
                        "failed to connect to local target"
                    );
                }
            }
        });
    }

    Err(anyhow::anyhow!(
        "SSH session ended while waiting for forwarded connections"
    ))
    .context("expose loop terminated unexpectedly")
}
