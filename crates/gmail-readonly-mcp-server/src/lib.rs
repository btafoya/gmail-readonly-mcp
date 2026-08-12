//! gmail-mcp-server: the MCP stdio server adapter over the core read-only
//! mail service.
//!
//! This crate owns MCP protocol details (tools, resources, prompts, error
//! mapping) and contains no Gmail logic.

pub mod error;
pub mod prompts;
pub mod resources;
pub mod server;

pub use server::GmailMcpServer;

use gmail_mcp_core::config::ConfigFile;
use rmcp::service::serve_server;
use rmcp::transport::stdio;

/// Run the MCP server over stdio until the client disconnects.
///
/// stdout is reserved for MCP protocol traffic; all logging must go to stderr.
pub async fn run_server(
    config: ConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let handler = GmailMcpServer::new(config);
    let service = serve_server(handler, stdio()).await?;
    service.waiting().await?;
    Ok(())
}
