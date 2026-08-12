//! The `serve` command: run the MCP server over stdio.

use gmail_mcp_core::config::ConfigFile;
use gmail_mcp_core::error::Error;

use crate::ServeArgs;

pub async fn run(args: ServeArgs) -> Result<(), Error> {
    // stdout is reserved for MCP protocol traffic; all logs go to stderr.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .init();

    let config = ConfigFile::load()?;
    gmail_readonly_mcp_server::run_server(config)
        .await
        .map_err(|e| Error::Internal(format!("server error: {e}")))?;
    Ok(())
}
