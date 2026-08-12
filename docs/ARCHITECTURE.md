# Architecture — gmail-mcp

## 1. Workspace

```text
gmail-mcp/
├── Cargo.toml
├── Cargo.lock
├── crates/
│   ├── gmail-mcp-core/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   │       └── fixtures/
│   ├── gmail-mcp-imap/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── tests/
│   │       └── fixtures/
│   ├── gmail-mcp-server/
│   │   ├── Cargo.toml
│   │   └── src/
│   └── gmail-mcp-cli/
│       ├── Cargo.toml
│       └── src/
└── tests/
```

## 2. Dependency direction

```text
gmail-mcp-cli ────────┐
                      ├──> gmail-mcp-core ───> abstractions
gmail-mcp-server ─────┘
                              ^
                              |
                     gmail-mcp-imap
```

The core crate must not depend on the concrete IMAP crate.

The CLI and MCP server are adapters.

## 3. Core

`gmail-mcp-core` contains:

- domain models
- account/configuration types
- mail service traits
- message/thread/mailbox abstractions
- application IDs
- search models
- MIME models
- rendering abstractions
- attachment service abstractions
- cache interfaces/implementation
- date/time normalization
- error types

It should contain no MCP-specific protocol types and no IMAP-specific implementation details.

## 4. IMAP crate

`gmail-mcp-imap` implements the core mail service using a mature Rust IMAP library.

Responsibilities:

- TLS connection
- authentication
- connection lifecycle
- mailbox discovery
- message search
- message retrieval
- header retrieval
- MIME retrieval
- attachment retrieval
- Gmail extensions
- Gmail unified search where practical
- read-only enforcement

The IMAP dependency must be isolated behind the core interfaces.

## 5. MCP server

`gmail-mcp-server` adapts the core service to MCP.

It owns:

- MCP initialization
- stdio transport
- tool registration
- resource registration
- prompt registration
- protocol serialization
- mapping domain errors into MCP errors

It must not implement Gmail logic.

## 6. CLI

`gmail-mcp-cli` owns:

- `clap`
- command hierarchy
- interactive config wizard
- human output
- output-format selection
- process startup

It calls the same core service used by the MCP server.

## 7. Configuration

Configuration is loaded by core configuration code.

The default file is:

`~/.config/gmail-mcp.toml`

The configuration loader should:

- resolve the XDG config location appropriately
- create the file/directories when needed
- parse TOML
- validate accounts
- enforce/warn about `0600`
- support interactive creation through the CLI

## 8. Mail service abstraction

Define a read-only service interface conceptually similar to:

```text
MailService
├── list_mailboxes
├── get_mailbox
├── get_mailbox_status
├── search_messages
├── get_message
├── get_thread
├── list_attachments
├── get_attachment
└── get_headers
```

There must be no mutation methods in the trait.

This is a deliberate architectural security boundary.

## 9. Message model

A message should contain:

- stable application ID
- account alias
- mailbox/labels
- provider IDs
- flags
- headers
- sender
- recipients
- timestamps
- MIME structure
- plain text
- HTML
- sanitized HTML
- Markdown
- attachment metadata

## 10. Application IDs

Prefer Gmail `X-GM-MSGID`.

Generate a deterministic opaque application ID from stable provider identifiers.

Use IMAP identifiers as fallback.

The application ID must not expose unnecessary provider implementation details.

## 11. Thread model

Prefer Gmail `X-GM-THRID`.

Use RFC header relationships as fallback/supporting information.

Use subject normalization only as a final fallback.

The thread model should retain enough metadata to explain how the relationship was determined when useful.

## 12. Search

The search abstraction should support:

- structured filters
- Gmail-native query
- mailbox constraints
- date filters
- sender/recipient
- subject
- text
- attachment presence
- flags

The concrete Gmail implementation may translate structured filters to Gmail search or IMAP criteria.

## 13. Rendering

Use internal abstractions for:

```text
HtmlRenderer
MessageRenderer
```

Do not let the chosen third-party HTML-to-Markdown library leak into the domain API.

## 14. Attachments

Attachment retrieval should return a domain-level result that can represent:

- inline content
- temporary file path
- metadata

The 25 MB direct-delivery threshold is an application policy, not an IMAP implementation detail.

## 15. Cache

Use a process-local in-memory cache.

Cache keys should use stable application identifiers where possible.

No persistent storage layer is required.

## 16. Connections

The IMAP implementation should reuse connections when sensible and reconnect stale connections.

Do not introduce a general connection pool unless implementation experience demonstrates that it is required.

## 17. Logging

Use `tracing`.

Sensitive data must never be emitted.

## 18. Error model

Define stable application error categories:

- configuration
- authentication
- account_not_found
- mailbox_not_found
- message_not_found
- thread_not_found
- attachment_not_found
- invalid_request
- imap
- mime
- network
- timeout
- internal

Include retryability where meaningful.

## 19. Runtime

Choose the current mature async runtime appropriate for the selected IMAP and MCP libraries. Do not force a runtime into core domain code.

## 20. Dependency policy

Prefer mature, actively maintained crates.

Avoid implementing standards or parsers ourselves when an appropriate mature crate exists.
