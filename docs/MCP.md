# MCP Interface — gmail-mcp

## Transport

The server communicates using MCP over `stdio`.

The process is intended to be launched by Claude Code and remain alive for the MCP session.

## Tools

The initial tool surface is intentionally small.

### `list_mailboxes`

Lists available mailboxes for an account.

Purpose:

- discover normalized Gmail mailboxes
- discover other labels/mailboxes
- retrieve mailbox metadata

### `get_mailbox`

Gets details for a normalized mailbox name.

### `get_mailbox_status`

Returns:

- total
- unread
- recent

### `search_messages`

Searches mail.

Supports both:

1. Structured search filters.
2. Gmail-native search syntax.

Structured filters may include:

- account
- mailbox
- sender
- recipients
- subject
- text
- date/time
- attachments
- flags

Search results are metadata-first.

Spam and Trash are excluded unless explicitly requested.

### `get_message`

Returns the complete message.

Default representation: JSON.

Supported formats:

- JSON
- Markdown
- HTML
- plain text
- raw MIME

Message retrieval must not mark messages `\Seen`.

### `get_thread`

Returns a conversation/thread.

Default behavior is a summary.

Summary includes:

- subject
- participants
- message count
- date range
- per-message summaries

Full retrieval returns complete messages chronologically.

Thread identity priority:

1. Gmail `X-GM-THRID`
2. RFC headers
3. subject normalization

### `list_attachments`

Returns attachment metadata for a message.

### `get_attachment`

Returns explicitly requested attachment content.

Files above 25 MB should be written to:

`~/.cache/gmail-mcp/attachments/`

The response must include metadata and the temporary path.

### `get_headers`

Returns message headers.

## Account selection

Every tool accepts an optional account alias.

If omitted:

- use configured default account
- if no default exists, return a structured error

Unknown aliases return `account_not_found` and available aliases.

## Resources

Expose read-only resources for:

- mailboxes
- messages
- threads
- attachments

Use a consistent URI namespace incorporating:

- account alias
- resource type
- opaque application ID where applicable

The exact URI syntax should be chosen during implementation based on the selected MCP SDK.

## Prompts

Provide prompts for:

### Email analysis

Examples of intended capabilities:

- summarize an email
- extract action items
- identify decisions
- identify requests
- identify important dates

### Thread analysis

Examples:

- summarize conversation
- identify participants
- identify decisions
- identify unresolved questions
- identify next steps

Prompts accept structured parameters plus arbitrary analysis instructions.

Prompts must remain scoped to read-only email analysis.

## MCP response principles

- Prefer structured JSON data for complex objects.
- Avoid unnecessary body content in search results.
- Use explicit retrieval for large content.
- Do not expose app passwords.
- Do not expose filesystem contents outside explicitly requested attachment paths.
- Do not expose arbitrary local filesystem paths.

## Read-only constraint

The MCP server must not expose any operation capable of:

- sending
- replying
- forwarding
- deleting
- moving
- copying
- flagging
- labeling
- marking read/unread
- otherwise mutating Gmail state.
