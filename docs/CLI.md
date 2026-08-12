# CLI Interface — gmail-mcp

## Binary

```text
gmail-mcp
```

## Command hierarchy

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

## Output

Human-readable output is the default.

Supported output representations:

- `--json`
- `--markdown`
- `--html`
- `--text`

Message retrieval additionally supports raw MIME.

## Account

Commands that operate on mail should accept an account alias.

If omitted, use the configured default account.

If no default exists, return an error.

## Messages search

The command should support:

- Gmail-native search query
- structured search options
- account
- mailbox
- sender
- recipients
- subject
- text
- date range
- attachment presence
- flags
- result limit/offset

Search results should default to metadata.

## Messages get

Returns a complete message.

Default:

```text
JSON
```

Formats:

```text
json
markdown
html
text
raw
```

Retrieval must not mark a message read.

## Threads get

Supports thread identification by application thread ID or appropriate message/thread reference.

Default output is a thread summary.

Full retrieval returns complete messages chronologically.

## Mailboxes

### list

Discover mailboxes.

### get

Get details for a normalized mailbox.

### status

Return:

- total
- unread
- recent

## Attachments

### list

List attachment metadata.

### get

Retrieve a selected attachment.

For attachments above 25 MB, write to the configured temporary attachment directory and print the path.

## Headers

Return message headers in a useful structured or human-readable form.

## Config add

Interactive wizard for adding a named account.

## Serve

Starts the MCP server over stdio.

The server should not emit normal logging to stdout because stdout is reserved for MCP protocol traffic. Logs belong on stderr.

## CLI design principle

The CLI is an adapter over the same core service used by MCP.

Do not duplicate business logic between CLI and server.
