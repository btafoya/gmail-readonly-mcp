//! The `setup` command: register gmail-mcp as an MCP server in Claude Code.

use gmail_mcp_core::config::default_config_path;
use gmail_mcp_core::error::Error;

/// The MCP server name registered with Claude Code.
const SERVER_NAME: &str = "gmail";

/// Register gmail-mcp with Claude Code via the `claude mcp add` command.
pub fn run() -> Result<(), Error> {
    let exe = std::env::current_exe()
        .map_err(|e| Error::Internal(format!("cannot locate the gmail-mcp binary: {e}")))?;

    // `claude` must be on PATH.
    if std::process::Command::new("claude")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("the `claude` CLI was not found on PATH.");
        eprintln!("Install Claude Code first, then register manually:");
        eprintln!("  claude mcp add {SERVER_NAME} -- {} serve", exe.display());
        return Err(Error::Internal("claude CLI not found".into()));
    }

    let status = std::process::Command::new("claude")
        .args(["mcp", "add", SERVER_NAME, "--"])
        .arg(&exe)
        .arg("serve")
        .status()
        .map_err(|e| Error::Internal(format!("failed to run `claude mcp add`: {e}")))?;

    if !status.success() {
        return Err(Error::Internal("`claude mcp add` failed".into()));
    }

    println!("Registered gmail-mcp with Claude Code as `{SERVER_NAME}`.");
    println!("Verify with: claude mcp list");
    println!("Restart Claude Code for the change to take effect.");

    if !default_config_path().exists() {
        println!();
        println!("No account configured yet. Run `gmail-mcp config add` to set one up.");
    }
    Ok(())
}
