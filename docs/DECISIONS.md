# Architecture Decision Record Summary

This file records the decisions made during requirements gathering.

## Authentication

Gmail App Password over IMAP.

## Credential storage

Plaintext app password in local TOML configuration.

## MCP transport

Local `stdio`.

## Account model

Multiple accounts selected by configured alias.

## Default account

Optional `default = true` within an account table.

## Configuration

`~/.config/gmail-mcp.toml`.

## Configuration format

TOML.

## Project structure

Cargo workspace with:

- `gmail-mcp-core`
- `gmail-mcp-imap`
- `gmail-mcp-server`
- `gmail-mcp-cli`

## CLI

Full CLI mirroring MCP capabilities plus `serve`.

Resource-oriented explicit command nouns:

```text
messages
threads
mailboxes
attachments
headers
config
```

## CLI framework

`clap`.

## IMAP implementation

Mature Rust IMAP crate behind an application-owned abstraction.

## MCP implementation

Mature Rust MCP crate behind an application-owned abstraction.

## Read-only

Maximum defense-in-depth enforcement.

## Search

Structured search + Gmail-native search syntax.

## Threading

Gmail native thread ID preferred; RFC headers and subject normalization are fallback mechanisms.

## Message IDs

Stable opaque application IDs using Gmail message IDs where available.

## Search output

Metadata by default.

## Message output

JSON by default.

## Message formats

- JSON
- Markdown
- HTML
- plain text
- raw MIME

## HTML

Return raw and sanitized HTML.

## HTML external resources

Disabled by default with explicit allowlisting if support is added.

## MIME

Full MIME parsing and CID/inline resource resolution.

## Attachments

Explicit retrieval.

25 MB direct-response threshold.

Temporary files:

`~/.cache/gmail-mcp/attachments/`

Default retention: 24 hours.

## Cache

Process-local memory only.

## Spam/Trash

Discoverable and explicitly searchable, but excluded from ordinary searches.

## Mailbox status

Total, unread, recent.

## Dates

Natural-language dates plus ISO 8601.

Rust-side deterministic date handling; per-account timezone with system fallback.

## Tests

Offline only. Synthetic fixtures. No live Gmail tests.

## Dependencies

Mature crates preferred.

## Scope exclusions

No:

- sending
- mutation
- OAuth
- web UI
- remote MCP
- persistent mail database
- embedded LLM
- generalized provider implementation
