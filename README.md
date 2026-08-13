# gmail-mcp

[![crates.io](https://img.shields.io/crates/v/gmail-readonly-mcp-server.svg)](https://crates.io/crates/gmail-readonly-mcp-server)
[![downloads](https://img.shields.io/crates/d/gmail-readonly-mcp-server.svg)](https://crates.io/crates/gmail-readonly-mcp-server)
[![license](https://img.shields.io/crates/l/gmail-readonly-mcp-server.svg)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.97%2B-orange.svg)](https://www.rust-lang.org)
[![stars](https://img.shields.io/github/stars/btafoya/gmail-readonly-mcp.svg)](https://github.com/btafoya/gmail-readonly-mcp)

A local Rust CLI and MCP server that gives a Claude Code agent strictly
**read-only** access to one or more Gmail accounts over IMAP.

- **Read-only by construction** — mailboxes are opened with `EXAMINE`, bodies
  are fetched with `BODY.PEEK`, and the service trait has no mutation methods.
  Retrieving a message never marks it `\Seen`.
- **Local-first** — runs on your machine; credentials stay in a local TOML
  file; no cloud backend, no persistent mail cache.
- **Two interfaces, one service layer** — a human-facing CLI and an MCP
  `stdio` server both call the same read-only `MailService`.

## Features

- Search mail with Gmail-native syntax or structured filters
- Retrieve complete messages as JSON, Markdown, HTML, plain text, or raw MIME
- Follow conversations/threads (Gmail `X-GM-THRID` takes precedence)
- Inspect mailboxes, headers, and MIME structure
- List and retrieve attachments (files over 25 MB spill to a temp cache)
- Multiple named accounts with a configurable default
- Interactive first-run configuration wizard

## Install

### From crates.io

```bash
cargo install gmail-mcp-cli
# binary lands at ~/.cargo/bin/gmail-mcp
```

### From source

```bash
cargo install --path crates/gmail-mcp-cli
# or: cargo install --git https://github.com/btafoya/gmail-readonly-mcp.git gmail-mcp-cli
```

## Quick start

```bash
# 1. Configure an account (interactive wizard; writes ~/.config/gmail-mcp.toml)
gmail-mcp config add

# 2. Register the MCP server with Claude Code
gmail-mcp setup

# 3. Use the CLI directly
gmail-mcp mailboxes list
gmail-mcp messages search --query "from:alice has:attachment"
gmail-mcp messages get <id> --markdown
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
gmail-mcp setup
gmail-mcp serve
```

Every mail command accepts `--account <alias>`; when omitted, the configured
default account is used.

## MCP server

```bash
gmail-mcp serve
```

Runs the MCP server over `stdio` (stdout is reserved for protocol traffic;
logs go to stderr). `gmail-mcp setup` registers it with Claude Code for all
projects; to register manually:

```bash
claude mcp add --scope user gmail -- gmail-mcp serve
```

The server exposes 9 read-only tools, resource URIs
(`gmail://<account>/messages/<id>`, …), and email/thread analysis prompts.

## Configuration

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

The table name (`personal`) is the account alias. `default = true` marks the
account used when none is specified. See [docs/CONFIG.md](docs/CONFIG.md) for
the full schema.

## Security

- Credentials are never logged or returned through MCP.
- HTML is sanitized (scripts, forms, event handlers removed); external
  resources are disabled by default.
- Attachment filenames are sanitized; files over 25 MB spill to
  `~/.cache/gmail-mcp/attachments/` with 24-hour retention.
- No write capability exists anywhere in the codebase.

## Workspace

```
crates/
├── gmail-mcp-core/    domain models, config, read-only service trait, rendering
├── gmail-mcp-imap/    concrete IMAP implementation (async-imap + TLS)
├── gmail-readonly-mcp-server/  MCP stdio server (tools, resources, prompts)
└── gmail-mcp-cli/     clap CLI + interactive config wizard + `serve`
```

## Documentation

The full specification lives in [`docs/`](docs/):

- [PRD](docs/PRD.md) — product requirements
- [Architecture](docs/ARCHITECTURE.md) — crate layout and design
- [MCP interface](docs/MCP.md) — tools, resources, prompts
- [Configuration](docs/CONFIG.md) — config schema and validation
- [CLI](docs/CLI.md) — command reference
- [Security model](docs/SECURITY.md) — threat model and guarantees
- [Testing](docs/TESTING.md) — test strategy
- [Implementation](docs/IMPLEMENTATION.md) — build plan and status
- [Decisions](docs/DECISIONS.md) — architecture decision record

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to
build, test, and submit changes.

## License

MIT — see [LICENSE](LICENSE).
