# Product Requirements — gmail-mcp

## 1. Purpose

`gmail-mcp` is a local Rust application that gives an AI agent, particularly Claude Code, controlled read-only access to Gmail through IMAP.

It provides both:

1. A human-facing CLI.
2. A local MCP server using `stdio`.

Both interfaces use the same application/service layer.

## 2. Primary goals

- Read Gmail messages without changing mailbox state.
- Search Gmail efficiently.
- Retrieve complete messages.
- Understand complete email conversations/threads.
- Inspect headers and MIME structure.
- List and retrieve attachments.
- Present mail as structured JSON or rendered text formats.
- Support multiple Gmail accounts.
- Keep credentials local.
- Keep the process local.
- Avoid persistent copies of the mailbox.

## 3. Authentication

Authentication is Gmail IMAP using an application password.

OAuth is not required.

Credentials are stored in the local TOML configuration.

## 4. Configuration

Default configuration:

`~/.config/gmail-mcp.toml`

Accounts are named TOML tables:

```toml
[accounts.personal]
email = "me@gmail.com"
app_password = "..."
imap_host = "imap.gmail.com"
imap_port = 993
tls = true
timezone = "America/New_York"
default = true
```

The table name (`personal`) is the account alias. Do not duplicate it as a `label` field.

The application should automatically create/configure the file on first run using an interactive wizard.

`gmail-mcp config add` launches the same wizard for another account.

The config file should be changed to mode `0600` where possible. If this cannot be enforced, warn the user.

## 5. Accounts

Multiple accounts are supported.

MCP requests may select an account by alias.

If an account is omitted, use the configured default account. If there is no default, return an error.

If an unknown account is requested, return a structured `account_not_found` error and list available aliases.

## 6. Read-only requirement

The system is strictly read-only.

No operations may:

- send mail
- reply
- forward
- delete
- move
- copy
- alter labels
- alter flags
- mark messages read/unread
- modify Gmail state

The read-only property must be enforced in:

1. MCP surface.
2. Core service abstraction.
3. IMAP implementation.

Message retrieval must avoid causing `\Seen`.

## 7. Mailbox functionality

Provide:

- mailbox discovery
- mailbox details
- mailbox status

Common Gmail mailboxes should have normalized logical names while retaining raw IMAP names.

Spam and Trash are discoverable/readable but excluded from normal searches unless explicitly requested.

Mailbox status focuses on:

- total
- unread
- recent

## 8. Message functionality

Provide:

- message search
- message retrieval
- header inspection

Search supports:

- structured filters
- Gmail-native search syntax

Normal searches use Gmail's unified search view where available.

Search uses Gmail/IMAP relevance where supported rather than implementing an LLM or custom relevance engine.

Search results are metadata-first. Message content is retrieved explicitly.

## 9. Message representations

The application supports:

- JSON
- Markdown
- HTML
- plain text
- raw MIME

JSON is the default for message retrieval.

JSON includes the complete message representation:

- application ID
- provider IDs where available
- mailbox/labels
- flags
- headers
- sender/recipients
- dates
- MIME structure
- body representations
- attachment metadata

## 10. Threads

Thread retrieval is first-class.

Default thread retrieval is a summary rather than full message bodies.

The summary includes:

- subject
- participants
- message count
- date range
- short per-message summaries

A full thread request returns complete messages in chronological order.

Thread correlation priority:

1. Gmail `X-GM-THRID`
2. RFC `Message-ID`, `In-Reply-To`, `References`
3. Subject normalization

When Gmail's native thread ID is available, it wins.

When ordering messages, reconstruct reply hierarchy first and then use chronology.

## 11. MIME

Use a mature MIME parser.

Support:

- plain text
- HTML
- multipart/alternative
- multipart/mixed
- nested MIME
- attachments
- inline resources
- CID references
- inline images

HTML-to-Markdown conversion must be behind an internal renderer abstraction.

## 12. HTML safety

Return both:

- original raw HTML
- sanitized HTML

Sanitization removes active/executable content such as:

- scripts
- forms
- event handlers

while retaining useful email formatting.

External resources are disabled by default. If support is added, it must use an explicit allowlist/configuration policy.

## 13. Attachments

Attachment metadata is always available.

Contents are retrieved only when explicitly requested.

Attachments up to 25 MB may be returned directly through MCP.

Larger attachments are written to:

`~/.cache/gmail-mcp/attachments/`

Attachment filenames must be sanitized while preserving useful names.

Temporary attachments default to 24-hour retention. The retention period is configurable.

## 14. Caching

Use short-lived in-memory caching only.

Do not create:

- SQLite mailbox cache
- persistent message cache
- local mailbox mirror

## 15. MCP

Transport: `stdio`.

The server exposes:

### Tools

- `list_mailboxes`
- `get_mailbox`
- `get_mailbox_status`
- `search_messages`
- `get_message`
- `get_thread`
- `list_attachments`
- `get_attachment`
- `get_headers`

### Resources

Expose read-only mailbox, message, thread, and attachment resources using a consistent URI scheme.

### Prompts

Provide:

- generic email-analysis prompts
- thread/conversation-analysis prompts

Prompts accept structured parameters and arbitrary additional analysis instructions, while remaining scoped to email analysis.

## 16. CLI

Command structure:

```text
gmail-mcp messages search
gmail-mcp messages get
gmail-mcp threads get
gmail-mcp mailboxes list
gmail-mcp mailboxes get
gmail-mcp mailboxes status
gmail-mcp attachments list
gmail-mcp attachments get
gmail-mcp headers get
gmail-mcp config add
gmail-mcp serve
```

Output formats:

- human-readable default
- `--json`
- `--markdown`
- `--html`
- `--text`

Raw MIME is available explicitly for message retrieval.

## 17. Testing

All tests are offline.

Use:

- unit tests
- fake/mock IMAP integration tests
- MCP protocol tests
- MIME/rendering fixtures

Fixtures must be synthetic.

No live Gmail integration tests.

## 18. Explicit non-goals

Do not add:

- SMTP
- email sending
- mutation operations
- OAuth
- web UI
- remote MCP
- background daemon
- persistent email database
- embedded LLM
- generalized multi-provider support
