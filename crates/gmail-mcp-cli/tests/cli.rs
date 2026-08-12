//! CLI tests: command parsing, help output, and configuration handling.
//! No live Gmail.

use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_gmail-mcp")
}

fn run(args: &[&str], config_home: Option<&str>) -> std::process::Output {
    let mut cmd = Command::new(bin());
    cmd.args(args);
    if let Some(home) = config_home {
        cmd.env("XDG_CONFIG_HOME", home);
    }
    // Non-interactive: stdin is null so the first-run wizard fails fast.
    cmd.stdin(Stdio::null());
    cmd.output().unwrap()
}

#[test]
fn help_exits_zero_and_lists_commands() {
    let out = run(&["--help"], None);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for cmd in [
        "messages",
        "threads",
        "mailboxes",
        "attachments",
        "headers",
        "config",
        "serve",
    ] {
        assert!(text.contains(cmd), "help missing {cmd}");
    }
}

#[test]
fn subcommand_help_shows_options() {
    let out = run(&["messages", "search", "--help"], None);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    for flag in [
        "--query",
        "--sender",
        "--subject",
        "--body",
        "--limit",
        "--json",
    ] {
        assert!(text.contains(flag), "search help missing {flag}");
    }
}

#[test]
fn serve_without_config_errors_helpfully() {
    let dir = std::env::temp_dir().join(format!("gmail-mcp-cli-serve-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = run(&["serve"], Some(dir.to_str().unwrap()));
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("no configuration file") || err.contains("config add"),
        "unexpected error: {err}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn invalid_subcommand_fails() {
    let out = run(&["bogus"], None);
    assert!(!out.status.success());
}

#[test]
fn messages_get_without_config_fails_cleanly() {
    // Non-interactive: the first-run wizard cannot run without a TTY, so the
    // command must fail rather than hang.
    let dir = std::env::temp_dir().join(format!("gmail-mcp-cli-get-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = run(&["messages", "get", "m:1"], Some(dir.to_str().unwrap()));
    assert!(!out.status.success());
    std::fs::remove_dir_all(&dir).unwrap();
}
