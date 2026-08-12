# gmail-mcp

A local Rust CLI and MCP server that gives a Claude Code agent strictly
**read-only** access to one or more Gmail accounts over IMAP.

- **Read-only by construction** — mailboxes are opened with `EXAMINE`, bodies
  are fetched with `BODY.PEEK`, and the service trait has no mutation methods.
  Retrieving a message never marks it `\Seen`.
- **Local-first** — runs on your machine; credentials stay in a local TOML
  file; no cloud backend, no persistent mail cache.
- **Two interfaces, one service layer** — a human-facing CLI and an MCP
  `stdio` server both call the same read-only `MailService`.

## Workspace

```
crates/
├── gmail-mcp-core/    domain models, config, read-only service trait, rendering
├── gmail-mcp-imap/    concrete IMAP implementation (async-imap + TLS)
├── gmail-readonly-mcp-server/  MCP stdio server (tools, resources, prompts)
└── gmail-mcp-cli/     clap CLI + interactive config wizard + `serve`
```

## Install

### From source with `cargo install`

```bash
cargo install --path crates/gmail-mcp-cli
# binary lands at ~/.cargo/bin/gmail-mcp
```

Or directly from the repository:

```bash
cargo install --git https://github.com/btafoya/gmail-readonly-mcp.git gmail-mcp-cli
```

### Register with Claude Code

```bash
gmail-mcp setup
```

This runs `claude mcp add --scope user gmail -- <path-to-gmail-mcp> serve`,
registering the server with Claude Code for all projects. Use
`--scope project` to register it in the current repo's `.mcp.json`, or
`--scope local` to limit it to the current project. Verify with
`claude mcp list`, then restart Claude Code. To register manually:

```bash
claude mcp add --scope user gmail -- gmail-mcp serve
```

## Setup

```bash
gmail-mcp config add        # interactive wizard; writes ~/.config/gmail-mcp.toml
```

Configuration lives at `~/.config/gmail-mcp.toml` (respecting
`XDG_CONFIG_HOME`), protected with mode `0600`:

```toml
[accounts.personal]
email = "me@gmail.com"
app_password = "xxxx xxxx xxxx xxxx"
imap_host = "imap.gmail.com"
imap_port = 993
tls = true
default = true
```

## CLI

```text
gmail-mcp messages search [--query ...] [--sender ...] [--subject ...] [--json]
gmail-mcp messages get <id> [--markdown|--html|--text|--raw]
gmail-mcp threads get <id> [--full]
gmail-mcp mailboxes list|get|status
gmail-mcp attachments list <message-id> | get <attachment-id>
gmail-mcp headers get <id>
gmail-mcp config add
gmail-mcp serve
```

## MCP server

```bash
gmail-mcp serve
```

Runs the MCP server over `stdio` (stdout is reserved for protocol traffic;
logs go to stderr). Configure it in Claude Code:

```json
{
  "mcpServers": {
    "gmail": { "command": "gmail-mcp", "args": ["serve"] }
  }
}
```

The server exposes 9 read-only tools, resource URIs
(`gmail://<account>/messages/<id>`, …), and email/thread analysis prompts.

## Security

- Credentials are never logged or returned through MCP.
- HTML is sanitized (scripts, forms, event handlers removed); external
  resources are disabled by default.
- Attachment filenames are sanitized; files over 25 MB spill to
  `~/.cache/gmail-mcp/attachments/` with 24-hour retention.
- No write capability exists anywhere in the codebase.

See `docs/` for the full specification.
