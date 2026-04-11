use tokio::io::{self, AsyncRead, AsyncWrite};
use tracing::info;

pub async fn bridge<L, R>(mut local: L, mut remote: R) -> anyhow::Result<()>
where
    L: AsyncRead + AsyncWrite + Unpin,
    R: AsyncRead + AsyncWrite + Unpin,
{
    let (sent, received) = io::copy_bidirectional(&mut local, &mut remote).await?;
    info!(sent, received, "bridge completed");
    Ok(())
}
