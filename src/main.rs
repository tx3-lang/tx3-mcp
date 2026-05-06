use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

mod args;
mod diagnostics;
mod server;
mod tii_emit;
mod tools;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn handle_cli_flags() -> Option<()> {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                // tx3up parses the LAST whitespace-separated token of stdout
                // as a semver Version (see up/src/bin.rs). Keep this format
                // stable: `<binary-name> <version>\n`.
                println!("tx3-mcp {VERSION}");
                return Some(());
            }
            "--help" | "-h" => {
                println!("tx3-mcp {VERSION} — MCP server for the Tx3 toolchain");
                println!();
                println!("USAGE:");
                println!("    tx3-mcp              Run the MCP server on stdio (default)");
                println!("    tx3-mcp --version    Print version and exit");
                println!("    tx3-mcp --help       Print this help and exit");
                return Some(());
            }
            _ => {}
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    if handle_cli_flags().is_some() {
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("tx3-mcp starting (version {VERSION})");

    let service = server::Tx3Server::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {e:?}");
        })?;

    service.waiting().await?;
    Ok(())
}
