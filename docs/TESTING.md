# Testing Strategy — gmail-mcp

## Core principles

- Tests are deterministic.
- Tests are offline.
- No real Gmail credentials.
- No real mailbox data.
- No live Gmail integration tests.
- Synthetic email fixtures only.

## Unit tests

Test:

- TOML parsing
- config validation
- account selection
- default account behavior
- application ID generation
- date parsing
- timezone handling
- search translation
- thread reconstruction
- MIME parsing
- HTML sanitization
- HTML-to-Markdown conversion
- attachment filename sanitization
- attachment threshold behavior
- retention cleanup
- error classification

## Fake IMAP tests

Use a fake/mock IMAP implementation behind the core interface.

Verify:

- mailbox discovery
- mailbox status
- message search
- message retrieval
- header retrieval
- attachment retrieval
- Gmail-specific ID handling
- reconnect behavior
- read-only behavior

## Read-only tests

Explicitly test that operations never invoke mutation behavior.

At minimum verify that:

- retrieving a message does not set `\Seen`
- no flag mutation occurs
- no move/copy/delete occurs
- no label mutation occurs

## MCP tests

Test:

- server startup
- tool registration
- tool argument validation
- successful calls
- missing account
- missing message
- malformed IDs
- structured errors
- resources
- prompts
- read-only constraints

## CLI tests

Test:

- command parsing
- output format selection
- account selection
- invalid configuration
- missing default account
- configuration wizard logic where practical

## MIME fixtures

Use synthetic fixtures only.

Fixtures should cover:

- plain text
- HTML
- multipart/alternative
- multipart/mixed
- inline images
- CID references
- multiple attachments
- nested MIME
- unusual headers
- missing headers
- malformed-but-tolerable MIME

Place fixtures inside the relevant crate's test fixture directory.

## No live Gmail

Do not add tests requiring:

- Gmail credentials
- environment variables containing real credentials
- network access
- a real Gmail mailbox

This is a deliberate project requirement.

## Test commands

Use conventional Cargo commands:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Add MCP/integration-specific test commands only when the implementation requires them.
