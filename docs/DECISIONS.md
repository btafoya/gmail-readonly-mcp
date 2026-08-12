# Architecture Decision Record Summary

This file records the decisions made during requirements gathering.

## Implementation decisions

Decisions made while implementing the specification:

- **Async runtime:** `tokio` (required by the chosen MCP SDK and IMAP client).
  Core domain code is runtime-agnostic; the runtime lives in the adapters.
- **IMAP client:** `async-imap` 0.11 with `tokio-native-tls`. TLS is layered
  manually (async-imap has no built-in TLS).
- **MCP SDK:** `rmcp` 3.x (official Rust SDK) with the `#[tool_router]` /
  `#[tool_handler]` macros for tools, resources, and prompts.
- **MIME:** `mailparse`; **HTML sanitization:** `ammonia`; **HTML→Markdown:**
  `html2md`; **HTML→text:** `html2text`. All isolated behind the core
  `HtmlRenderer` / `MessageRenderer` traits.
- **X-GM-THRID:** async-imap parses `X-GM-THRID` but does not expose it through
  its `Fetch` API. The IMAP crate drives one manual `UID FETCH` via
  `Session::run_command` + `read_response`, capturing body, flags, and both
  Gmail IDs in a single round trip. This is the only place the raw IMAP
  response is read directly.
- **Application IDs:** reversible and opaque — `m:<hex(provider_id)>` (and
  `t:`/`a:` for threads/attachments). Hex of the provider ID is deterministic
  and decodable so the service can map an application ID back to the provider
  identifier without keeping state.
- **Read-only enforcement:** mailboxes are opened with `EXAMINE` and bodies
  fetched with `BODY.PEEK`, so retrieval never sets `\Seen`; the `MailService`
  trait has no mutation methods.
- **CLI flag spelling:** the search body-text filter is `--body` because the
  output-format flag `--text` already occupies the name.
- **First run:** non-`serve` commands launch the interactive wizard when no
  config exists; `serve` errors with a pointer to `gmail-mcp config add`
  because stdio cannot host an interactive prompt.

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
- `gmail-readonly-mcp-server`
- `gmail-mcp-cli`

> The MCP server crate is published as `gmail-readonly-mcp-server` because the
> name `gmail-mcp-server` is already taken on crates.io by an unrelated
> project. The in-repo directory and internal lib name follow the published
> name.

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
