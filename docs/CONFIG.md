# Configuration — gmail-mcp

## Location

Default:

```text
~/.config/gmail-mcp.toml
```

The application should follow normal XDG behavior where appropriate, while retaining this path as the documented default.

## Example

```toml
[accounts.personal]
email = "me@gmail.com"
app_password = "xxxx xxxx xxxx xxxx"

imap_host = "imap.gmail.com"
imap_port = 993
tls = true

timezone = "America/New_York"

default = true
```

## Account aliases

The TOML table name is the account alias.

Example:

```toml
[accounts.personal]
```

means:

```text
personal
```

Do not include:

```toml
label = "personal"
```

because it duplicates the table name.

## Required account settings

Each account requires:

- `email`
- `app_password`
- `imap_host`
- `imap_port`
- `tls`

Recommended:

- `timezone`
- `default`

## Connection settings

Connection-related settings may be configurable when useful, including sensible timeout/keepalive behavior.

Do not overpopulate the configuration schema with implementation details that don't provide operational value.

## TLS

The secure default is normal system certificate verification.

`tls = true` is the initial configuration model.

If future use cases require custom CAs or explicit insecure mode, add them deliberately rather than making insecure behavior easy.

## Credentials

The application password is plaintext in the TOML file by explicit project decision.

Protect the file with mode `0600` when possible.

If the application cannot enforce this, warn.

Never log the app password.

Never return the app password through MCP.

## First run

When the configuration file does not exist:

1. Launch an interactive terminal wizard.
2. Collect account details.
3. Write the TOML.
4. Apply restrictive file permissions.
5. Continue with the requested operation when appropriate.

## Add account

`gmail-mcp config add`

launches the same wizard and adds another named account.

No large config-management command suite is required.

## Default account

An account may contain:

```toml
default = true
```

If multiple accounts are marked default, configuration validation should reject the ambiguity.

If no account is default, MCP requests must specify an account.

## Timezone

Timezone is per-account.

If omitted, use the system timezone.

Use the timezone for deterministic local date interpretation.

## Attachment settings

Attachment storage defaults to:

```text
~/.cache/gmail-mcp/attachments/
```

Default direct-delivery threshold:

```text
25 MB
```

Default retention:

```text
24 hours
```

Both may be configurable.

## External resources

External HTML resources are disabled by default.

If support is implemented, require an explicit allowlist/configuration.

## Validation

Configuration validation should fail for:

- malformed TOML
- missing required account fields
- invalid port
- invalid TLS configuration
- invalid timezone
- multiple default accounts
- unusable account aliases

Network authentication should be deferred until an account is actually used.
