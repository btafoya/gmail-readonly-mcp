# Security Model — gmail-mcp

## Threat model

The application gives a local AI agent access to email.

The primary security objective is:

> The agent can read email, but cannot cause Gmail state changes or access unrelated local/network resources through this application.

## Credential security

The Gmail app password is stored in the local TOML configuration by design.

Requirements:

- attempt `0600` permissions
- never log credentials
- never return credentials through MCP
- never include credentials in errors
- do not place credentials in URLs
- avoid accidental credential exposure in debug output

## Read-only enforcement

Read-only is enforced at multiple levels.

### MCP

No mutation tools.

### Core

No mutation methods in the mail service interface.

### IMAP

Never issue write operations.

Use read-only mailbox access mechanisms where available.

Message retrieval must not cause `\Seen`.

## Network boundary

The application should only connect to configured IMAP endpoints.

Do not implement arbitrary HTTP fetching.

HTML external resources are disabled by default.

If external resource support is ever added, require an explicit allowlist.

## Filesystem boundary

The application should access only:

- configuration file
- configured/standard attachment cache directory
- explicitly requested temporary attachment output

It must not expose arbitrary filesystem reads through MCP.

## Attachment security

Attachment filenames are untrusted.

Sanitize:

- `/`
- `\`
- `..`
- control characters
- unsafe path characters

Ensure final paths remain under the attachment cache directory.

## HTML security

Raw HTML is retained for fidelity.

Sanitized HTML must remove active content such as:

- scripts
- forms
- event handlers
- dangerous URL schemes

Do not automatically fetch external resources.

## Logging

Never log:

- app passwords
- complete email bodies
- attachment contents
- raw MIME

Debug logs should remain useful without exposing message data.

## Temporary data

Attachments above 25 MB are written to:

`~/.cache/gmail-mcp/attachments/`

Default retention is 24 hours.

Expired files are removed automatically.

## No persistent mail store

Do not create a persistent database containing the mailbox.

Only process-local memory caching is permitted.

## MCP safety

Tool arguments must be validated before accessing IMAP.

Reject:

- invalid account aliases
- invalid IDs
- malformed mailbox names
- unsafe attachment paths
- unsupported operations

## Security philosophy

Prefer structural prevention over documentation.

If an operation is not supposed to exist, do not expose it as a tool or service method.
