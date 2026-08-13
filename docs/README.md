# gmail-mcp — Documentation

The specification for the gmail-mcp project. Read these in order to understand
the full design.

| Document | Purpose |
|----------|---------|
| [PRD.md](PRD.md) | Product requirements: goals, scope, and explicit non-goals |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Crate layout, dependency direction, and design principles |
| [MCP.md](MCP.md) | The MCP interface: tools, resources, prompts, and response principles |
| [CONFIG.md](CONFIG.md) | Configuration schema, validation, and account model |
| [CLI.md](CLI.md) | The command-line interface and output formats |
| [SECURITY.md](SECURITY.md) | Threat model, read-only enforcement, and security guarantees |
| [TESTING.md](TESTING.md) | Testing strategy: offline, synthetic fixtures only |
| [IMPLEMENTATION.md](IMPLEMENTATION.md) | Build plan and current status |
| [DECISIONS.md](DECISIONS.md) | Architecture decision record, including implementation choices |

## Quick orientation

- **Read-only at every layer** — the `MailService` trait in
  `gmail-mcp-core` has no mutation methods; the IMAP crate opens mailboxes
  with `EXAMINE` and fetches with `BODY.PEEK`.
- **Local-first** — credentials live in `~/.config/gmail-mcp.toml`; no cloud
  backend, no persistent mail cache.
- **One service, two adapters** — the CLI and MCP server both call the same
  core service.
