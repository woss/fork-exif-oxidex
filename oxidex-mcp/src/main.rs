use anyhow::Result;
use oxidex_mcp::server;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is reserved for JSON-RPC)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("OxiDex MCP Server starting...");

    server::run_server().await?;

    Ok(())
}
