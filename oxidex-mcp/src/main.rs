use anyhow::Result;
use oxidex_mcp::server;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("OxiDex MCP Server starting...");

    server::run_server().await?;

    Ok(())
}
