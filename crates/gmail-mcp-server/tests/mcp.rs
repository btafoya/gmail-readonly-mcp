//! MCP protocol tests: tool registration, error mapping, prompts, and
//! resource resolution. No live Gmail.

use std::sync::Arc;

use gmail_mcp_core::config::ConfigFile;
use gmail_mcp_core::error::Error;
use gmail_mcp_core::model::{Mailbox, MailboxStatus};
use gmail_mcp_core::search::SearchRequest;
use gmail_mcp_core::service::MailService;
use gmail_mcp_server::GmailMcpServer;
use rmcp::ErrorData;

fn config() -> ConfigFile {
    let toml = r#"
[accounts.personal]
email = "me@gmail.com"
app_password = "abcd efgh ijkl mnop"
imap_host = "imap.gmail.com"
default = true
"#;
    toml::from_str(toml).unwrap()
}

#[test]
fn error_mapping_is_structured() {
    let cases: Vec<(Error, &str)> = vec![
        (
            Error::AccountNotFound("nope".into(), "personal".into()),
            "account not found: nope (available: personal)",
        ),
        (
            Error::MailboxNotFound("inbox".into()),
            "mailbox not found: inbox",
        ),
        (
            Error::MessageNotFound("m:1".into()),
            "message not found: m:1",
        ),
        (Error::InvalidRequest("bad id".into()), "bad id"),
    ];
    for (err, needle) in cases {
        let data: ErrorData = gmail_mcp_server::error::ServerError(err).into();
        assert!(
            data.message.contains(needle),
            "expected `{needle}` in `{}`",
            data.message
        );
    }
}

#[test]
fn prompts_are_defined_and_resolvable() {
    let prompts = gmail_mcp_server::prompts::list_prompts();
    let names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"analyze_email"));
    assert!(names.contains(&"analyze_thread"));

    let mut args = serde_json::Map::new();
    args.insert("account".into(), serde_json::json!("personal"));
    args.insert("message_id".into(), serde_json::json!("m:1"));
    let result = gmail_mcp_server::prompts::get_prompt("analyze_email", &args).unwrap();
    assert_eq!(result.messages.len(), 1);
    match &result.messages[0].content {
        rmcp::model::ContentBlock::Text(t) => assert!(t.text.contains("m:1")),
        other => panic!("expected text content, got {other:?}"),
    }

    assert!(gmail_mcp_server::prompts::get_prompt("nope", &args).is_none());
}

#[test]
fn resource_uris_parse() {
    use gmail_mcp_server::resources::{ResourceTarget, parse_uri};
    assert_eq!(
        parse_uri("gmail://personal/messages/m:1"),
        Some(("personal".into(), ResourceTarget::Message("m:1".into())))
    );
    assert_eq!(
        parse_uri("gmail://personal/mailboxes"),
        Some(("personal".into(), ResourceTarget::Mailboxes))
    );
    assert_eq!(parse_uri("file:///etc/passwd"), None);
}

// ---------------------------------------------------------------------------
// Fake service for resource resolution tests
// ---------------------------------------------------------------------------

struct FakeService;

#[async_trait::async_trait]
impl MailService for FakeService {
    async fn list_mailboxes(&self) -> Result<Vec<Mailbox>, Error> {
        Ok(vec![Mailbox {
            name: "inbox".into(),
            raw_name: "INBOX".into(),
            attributes: vec![],
            delimiter: None,
        }])
    }
    async fn get_mailbox(&self, name: &str) -> Result<Mailbox, Error> {
        Err(Error::MailboxNotFound(name.into()))
    }
    async fn get_mailbox_status(&self, _name: &str) -> Result<MailboxStatus, Error> {
        Ok(MailboxStatus {
            total: 1,
            unread: 0,
            recent: 0,
        })
    }
    async fn search_messages(
        &self,
        _r: &SearchRequest,
    ) -> Result<Vec<gmail_mcp_core::model::MessageSummary>, Error> {
        Ok(vec![])
    }
    async fn get_message(&self, _id: &str) -> Result<gmail_mcp_core::model::Message, Error> {
        Err(Error::MessageNotFound("x".into()))
    }
    async fn get_headers(&self, _id: &str) -> Result<Vec<gmail_mcp_core::model::Header>, Error> {
        Ok(vec![])
    }
    async fn get_thread(
        &self,
        _id: &str,
        _full: bool,
    ) -> Result<gmail_mcp_core::model::Thread, Error> {
        Err(Error::ThreadNotFound("x".into()))
    }
    async fn list_attachments(
        &self,
        _id: &str,
    ) -> Result<Vec<gmail_mcp_core::model::AttachmentMeta>, Error> {
        Ok(vec![])
    }
    async fn get_attachment(
        &self,
        _id: &str,
    ) -> Result<gmail_mcp_core::model::AttachmentResult, Error> {
        Err(Error::AttachmentNotFound("x".into()))
    }
}

#[tokio::test]
async fn read_resource_resolves_mailboxes() {
    let value = gmail_mcp_server::resources::read_target(
        &FakeService,
        &gmail_mcp_server::resources::ResourceTarget::Mailboxes,
    )
    .await
    .unwrap();
    assert!(value.to_string().contains("inbox"));
}

#[tokio::test]
async fn server_constructs_with_config() {
    let _server = GmailMcpServer::new(config());
    let _ = Arc::new(_server);
}
