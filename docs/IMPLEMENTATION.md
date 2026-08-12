# Implementation Plan — gmail-mcp

## Phase 1 — workspace

Create:

```text
gmail-mcp/
├── Cargo.toml
└── crates/
    ├── gmail-mcp-core/
    ├── gmail-mcp-imap/
    ├── gmail-mcp-server/
    └── gmail-mcp-cli/
```

Use the latest stable Rust edition/toolchain available.

Select mature dependencies based on current ecosystem status.

## Phase 2 — core domain

Implement:

- account configuration
- application IDs
- mailbox model
- message model
- thread model
- attachment model
- headers
- flags
- search types
- representation formats
- errors
- read-only mail service trait

Do not implement IMAP here.

## Phase 3 — configuration

Implement:

- `~/.config/gmail-mcp.toml`
- TOML parsing
- validation
- default account
- per-account timezone
- permission handling
- attachment settings
- connection settings

Implement CLI wizard support through the CLI crate.

## Phase 4 — IMAP

Select a mature Rust IMAP implementation.

Wrap it behind the core mail service abstraction.

Implement:

- TLS
- app-password authentication
- connection lifecycle
- reconnect
- mailbox discovery
- mailbox status
- search
- message retrieval
- headers
- attachments
- Gmail extensions

Strictly avoid write operations.

## Phase 5 — MIME

Select mature MIME parsing infrastructure.

Implement complete MIME tree handling.

Support:

- plain text
- HTML
- multipart structures
- attachments
- inline content
- CID resources
- nested MIME

## Phase 6 — rendering

Implement:

- JSON representation
- plain text
- HTML
- sanitized HTML
- Markdown
- raw MIME

Create internal renderer abstractions so third-party libraries remain replaceable.

## Phase 7 — threads

Implement:

1. Gmail native thread ID
2. RFC header relationship fallback
3. subject normalization fallback

Implement:

- summary
- full retrieval
- chronological ordering
- reply hierarchy reconstruction

## Phase 8 — attachments

Implement:

- metadata
- explicit retrieval
- 25 MB direct-return threshold
- temporary file output
- filename sanitization
- 24-hour cleanup
- configurable retention

## Phase 9 — CLI

Implement `clap`.

Add:

```text
messages search
messages get
threads get
mailboxes list
mailboxes get
mailboxes status
attachments list
attachments get
headers get
config add
serve
```

Add output formats.

## Phase 10 — MCP

Select a mature Rust MCP SDK.

Implement stdio transport.

Register:

- tools
- resources
- prompts

Map all operations to core service methods.

Do not duplicate business logic.

## Phase 11 — tests

Implement:

- core unit tests
- fake IMAP tests
- MIME fixtures
- rendering tests
- thread tests
- MCP protocol tests
- CLI tests
- read-only tests

No live Gmail.

## Phase 12 — integration

Verify that Claude Code can launch:

```text
gmail-mcp serve
```

using stdio MCP.

Verify:

- configuration loading
- account selection
- mailbox discovery
- search
- message retrieval
- thread retrieval
- attachments
- resources
- prompts
- graceful shutdown

## Phase 13 — hardening

Run:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Review:

- credential exposure
- logging
- filesystem access
- network access
- read-only enforcement
- attachment path safety
- HTML sanitization
- MCP stdout/stderr separation

## Implementation rule

Do not stop to ask the user about routine implementation decisions that are already covered by this specification.

Use engineering judgment for:

- exact crate versions
- internal struct names
- function signatures
- MCP SDK details
- IMAP library details
- parser implementation
- test organization
- CLI flag spelling

Only surface decisions that materially change the requirements or security model.
