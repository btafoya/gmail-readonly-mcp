# Contributing to gmail-mcp

Thanks for your interest! This project is intentionally small and focused:
a read-only Gmail CLI and MCP server. Keep that scope in mind when proposing
changes.

## Getting started

```bash
# Build
cargo build --workspace

# Run the test suite
cargo nextest run --workspace   # or: cargo test --workspace

# Lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Project layout

- `crates/gmail-mcp-core` — domain models, configuration, the read-only
  `MailService` trait, and rendering. No IMAP or MCP details live here.
- `crates/gmail-mcp-imap` — the concrete IMAP implementation.
- `crates/gmail-readonly-mcp-server` — the MCP stdio server adapter.
- `crates/gmail-mcp-cli` — the CLI and interactive wizard.

The CLI and MCP server are adapters over the same core service. Business
logic belongs in `gmail-mcp-core` or `gmail-mcp-imap`, never in the adapters.

## What to work on

Good first contributions:

- Bug fixes with a failing test that reproduces them
- Missing test coverage (see `docs/TESTING.md`)
- Documentation improvements

Please **do not** propose:

- Write capabilities (sending, deleting, moving, flagging) — the project is
  read-only by design
- A persistent mail cache or database
- OAuth or generalized multi-provider support
- A web UI or remote MCP transport

## Testing

All tests are offline. No live Gmail credentials or network access are used.
Synthetic fixtures only. See `docs/TESTING.md` for the full strategy.

## Submitting changes

1. Fork the repository and create a feature branch.
2. Make your change, following the existing code style.
3. Add or update tests.
4. Run the full verification loop above — everything must pass.
5. Open a pull request describing the change and why.

## Code style

- Match the surrounding code: same comment density, naming, and idioms.
- Prefer the smallest change that works. No speculative abstractions.
- Reuse existing helpers and the standard library before adding dependencies.
- Keep the diff surgical — don't reformat unrelated code.

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](LICENSE).
