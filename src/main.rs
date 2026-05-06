use anyhow::Result;
use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

mod args;
mod diagnostics;
mod server;
mod tii_emit;
mod tools;

/// MCP server for the Tx3 toolchain.
///
/// Run with no arguments to start the server on stdio. tx3up reads the
/// `--version` output to detect installed components — clap's default
/// format (`tx3-mcp <semver>\n`) matches what tx3up's parser expects.
#[derive(Parser, Debug)]
#[command(name = "tx3-mcp", version, about, long_about = None)]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    Cli::parse();

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
