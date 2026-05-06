use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

mod args;
mod diagnostics;
mod server;
mod tii_emit;
mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("tx3-mcp starting (version {})", env!("CARGO_PKG_VERSION"));

    let service = server::Tx3Server::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {e:?}");
        })?;

    service.waiting().await?;
    Ok(())
}
