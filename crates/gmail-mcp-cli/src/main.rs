//! gmail-mcp CLI: a human-facing adapter over the same read-only mail service
//! used by the MCP server.

mod output;
mod serve;
mod setup;
mod wizard;

use std::sync::Arc;

use clap::{Args, Parser, Subcommand};
use gmail_mcp_core::config::{ConfigFile, default_config_path};
use gmail_mcp_core::error::Error;
use gmail_mcp_core::render::{DefaultRenderer, MessageRenderer, RenderFormat};
use gmail_mcp_core::search::{SearchFilters, SearchRequest};
use gmail_mcp_core::service::MailService;
use gmail_mcp_imap::ImapService;

use crate::output::*;

#[derive(Parser)]
#[command(
    name = "gmail-mcp",
    version,
    about = "Read-only Gmail access via IMAP (CLI and MCP server)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // clap arg structs vary in size; boxing hurts ergonomics
enum Command {
    /// Search and retrieve messages.
    Messages(MessagesArgs),
    /// Retrieve conversations/threads.
    Threads(ThreadsArgs),
    /// Discover and inspect mailboxes.
    Mailboxes(MailboxesArgs),
    /// List and retrieve attachments.
    Attachments(AttachmentsArgs),
    /// Inspect message headers.
    Headers(HeadersArgs),
    /// Manage configuration.
    Config(ConfigArgs),
    /// Run the MCP server over stdio.
    Serve(ServeArgs),
    /// Register gmail-mcp as an MCP server in Claude Code.
    Setup(SetupArgs),
}

#[derive(Args)]
struct MessagesArgs {
    #[command(subcommand)]
    cmd: MessagesCmd,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum MessagesCmd {
    /// Search messages (metadata-first).
    Search(SearchArgs),
    /// Get a complete message.
    Get(GetArgs),
}

#[derive(Args)]
struct ThreadsArgs {
    #[command(subcommand)]
    cmd: ThreadsCmd,
}

#[derive(Subcommand)]
enum ThreadsCmd {
    /// Get a thread (summary by default).
    Get(ThreadGetArgs),
}

#[derive(Args)]
struct MailboxesArgs {
    #[command(subcommand)]
    cmd: MailboxesCmd,
}

#[derive(Subcommand)]
enum MailboxesCmd {
    /// List mailboxes.
    List(AccountArgs),
    /// Get details for a normalized mailbox.
    Get(MailboxArgs),
    /// Get mailbox status (total, unread, recent).
    Status(MailboxArgs),
}

#[derive(Args)]
struct AttachmentsArgs {
    #[command(subcommand)]
    cmd: AttachmentsCmd,
}

#[derive(Subcommand)]
enum AttachmentsCmd {
    /// List attachment metadata for a message.
    List(IdArgs),
    /// Retrieve an attachment.
    Get(AttachmentGetArgs),
}

#[derive(Args)]
struct HeadersArgs {
    #[command(subcommand)]
    cmd: HeadersCmd,
}

#[derive(Subcommand)]
enum HeadersCmd {
    /// Get message headers.
    Get(IdArgs),
}

#[derive(Args)]
struct ConfigArgs {
    #[command(subcommand)]
    cmd: ConfigCmd,
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Interactively add a named account.
    Add,
}

#[derive(Args, Clone)]
struct AccountArgs {
    /// Account alias (defaults to the configured default account).
    #[arg(long)]
    account: Option<String>,
}

#[derive(Args, Clone)]
struct MailboxArgs {
    #[arg(long)]
    account: Option<String>,
    /// Normalized mailbox name (e.g. inbox, sent, all).
    name: String,
}

#[derive(Args, Clone)]
struct IdArgs {
    #[arg(long)]
    account: Option<String>,
    /// Application ID.
    id: String,
}

#[derive(Args, Clone)]
struct SearchArgs {
    #[arg(long)]
    account: Option<String>,
    /// Gmail-native search query (e.g. `from:alice has:attachment`).
    #[arg(long)]
    query: Option<String>,
    #[arg(long)]
    mailbox: Option<String>,
    #[arg(long)]
    sender: Option<String>,
    #[arg(long)]
    recipients: Option<String>,
    #[arg(long)]
    subject: Option<String>,
    /// Search message body text.
    #[arg(long)]
    body: Option<String>,
    /// ISO 8601 date, inclusive lower bound.
    #[arg(long)]
    from: Option<String>,
    /// ISO 8601 date, inclusive upper bound.
    #[arg(long)]
    to: Option<String>,
    #[arg(long)]
    has_attachment: bool,
    #[arg(long)]
    flag: Vec<String>,
    #[arg(long)]
    include_spam: bool,
    #[arg(long)]
    include_trash: bool,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    offset: Option<usize>,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Args, Clone)]
struct GetArgs {
    #[arg(long)]
    account: Option<String>,
    /// Application ID.
    id: String,
    #[command(flatten)]
    output: OutputArgs,
    /// Output raw MIME.
    #[arg(long, conflicts_with_all = ["json", "markdown", "html", "text"])]
    raw: bool,
}

#[derive(Args, Clone)]
struct ThreadGetArgs {
    #[arg(long)]
    account: Option<String>,
    /// Application thread ID.
    id: String,
    /// Return complete messages instead of a summary.
    #[arg(long)]
    full: bool,
    #[command(flatten)]
    output: OutputArgs,
}

#[derive(Args, Clone)]
struct AttachmentGetArgs {
    #[arg(long)]
    account: Option<String>,
    /// Application attachment ID.
    id: String,
}

#[derive(Args, Clone)]
struct ServeArgs {
    /// Log level (e.g. info, debug). Logs go to stderr.
    #[arg(long, default_value = "info")]
    log: String,
}

#[derive(Args, Clone)]
struct SetupArgs {
    /// Where to register: user (all projects), project (this repo's .mcp.json),
    /// or local (this project only).
    #[arg(long, value_parser = ["user", "project", "local"], default_value = "user")]
    scope: String,
}

/// Output format flags shared by commands.
#[derive(Args, Clone, Default)]
struct OutputArgs {
    #[arg(long, conflicts_with_all = ["markdown", "html", "text"])]
    json: bool,
    #[arg(long, conflicts_with_all = ["json", "html", "text"])]
    markdown: bool,
    #[arg(long, conflicts_with_all = ["json", "markdown", "text"])]
    html: bool,
    #[arg(long, conflicts_with_all = ["json", "markdown", "html"])]
    text: bool,
}

impl OutputArgs {
    fn format(&self) -> Option<RenderFormat> {
        if self.json {
            Some(RenderFormat::Json)
        } else if self.markdown {
            Some(RenderFormat::Markdown)
        } else if self.html {
            Some(RenderFormat::Html)
        } else if self.text {
            Some(RenderFormat::Text)
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Serve(args) => serve::run(args).await,
        Command::Setup(args) => setup::run(&args.scope),
        Command::Config(args) => match args.cmd {
            ConfigCmd::Add => wizard::add_account(),
        },
        Command::Messages(args) => match args.cmd {
            MessagesCmd::Search(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let request = SearchRequest {
                    query: args.query,
                    filters: SearchFilters {
                        mailbox: args.mailbox,
                        sender: args.sender,
                        recipients: args.recipients,
                        subject: args.subject,
                        text: args.body,
                        date_from: parse_date(args.from.as_deref())?,
                        date_to: parse_date(args.to.as_deref())?,
                        has_attachment: args.has_attachment.then_some(true),
                        flags: args.flag,
                        include_spam: args.include_spam,
                        include_trash: args.include_trash,
                    },
                    limit: args.limit,
                    offset: args.offset,
                };
                let results = service.search_messages(&request).await?;
                match args.output.format() {
                    Some(RenderFormat::Json) => print_json(&results),
                    Some(f) => {
                        let renderer = DefaultRenderer::new();
                        for r in &results {
                            println!("{}", renderer.render_message(&message_from_summary(r), f)?);
                        }
                    }
                    None => print_search(&results),
                }
                Ok(())
            }
            MessagesCmd::Get(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let message = service.get_message(&args.id).await?;
                let renderer = DefaultRenderer::new();
                if args.raw {
                    println!("{}", renderer.render_message(&message, RenderFormat::Raw)?);
                } else {
                    let format = args.output.format().unwrap_or(RenderFormat::Json);
                    println!("{}", renderer.render_message(&message, format)?);
                }
                Ok(())
            }
        },
        Command::Threads(args) => match args.cmd {
            ThreadsCmd::Get(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let thread = service.get_thread(&args.id, args.full).await?;
                let renderer = DefaultRenderer::new();
                let format = args.output.format().unwrap_or(RenderFormat::Json);
                println!("{}", renderer.render_thread(&thread, format)?);
                Ok(())
            }
        },
        Command::Mailboxes(args) => match args.cmd {
            MailboxesCmd::List(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let mailboxes = service.list_mailboxes().await?;
                print_mailboxes(&mailboxes);
                Ok(())
            }
            MailboxesCmd::Get(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let mailbox = service.get_mailbox(&args.name).await?;
                print_mailbox(&mailbox);
                Ok(())
            }
            MailboxesCmd::Status(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let status = service.get_mailbox_status(&args.name).await?;
                print_status(&args.name, &status);
                Ok(())
            }
        },
        Command::Attachments(args) => match args.cmd {
            AttachmentsCmd::List(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let attachments = service.list_attachments(&args.id).await?;
                print_attachments(&attachments);
                Ok(())
            }
            AttachmentsCmd::Get(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let result = service.get_attachment(&args.id).await?;
                print_attachment(&result);
                Ok(())
            }
        },
        Command::Headers(args) => match args.cmd {
            HeadersCmd::Get(args) => {
                let config = load_config()?;
                let service = build_service(&config, args.account.as_deref())?;
                let headers = service.get_headers(&args.id).await?;
                print_headers(&headers);
                Ok(())
            }
        },
    }
}

/// Load configuration, launching the interactive wizard on first run.
fn load_config() -> Result<ConfigFile, Error> {
    let path = default_config_path();
    if !path.exists() {
        eprintln!("no configuration found; setting up your first account");
        wizard::add_account()?;
    }
    ConfigFile::load()
}

fn build_service(config: &ConfigFile, account: Option<&str>) -> Result<Arc<ImapService>, Error> {
    let (account, alias) = config.resolve_account(account)?;
    let policy = config.attachment_policy();
    let renderer = Arc::new(DefaultRenderer::new());
    Ok(Arc::new(ImapService::new(
        alias,
        account.clone(),
        policy,
        renderer,
    )))
}

fn parse_date(s: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>, Error> {
    match s {
        None => Ok(None),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|_| Error::InvalidRequest(format!("invalid date `{s}`; use ISO 8601"))),
    }
}

fn message_from_summary(
    s: &gmail_mcp_core::model::MessageSummary,
) -> gmail_mcp_core::model::Message {
    gmail_mcp_core::model::Message {
        id: s.id.clone(),
        thread_id: s.thread_id.clone(),
        account: s.account.clone(),
        mailbox: s.mailbox.clone(),
        provider: Default::default(),
        flags: s.flags.clone(),
        headers: vec![],
        sender: s.sender.clone(),
        to: vec![],
        cc: vec![],
        bcc: vec![],
        reply_to: vec![],
        subject: s.subject.clone(),
        date: s.date,
        size: 0,
        labels: vec![],
        mime_structure: None,
        plain_text: s.snippet.clone(),
        html: None,
        sanitized_html: None,
        markdown: None,
        attachments: vec![],
        raw_mime: None,
    }
}
