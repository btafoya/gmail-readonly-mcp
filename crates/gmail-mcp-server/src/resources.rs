//! Read-only MCP resource URIs.
//!
//! URI scheme: `gmail://<account>/<kind>/<id>` where `kind` is one of
//! `mailboxes`, `messages`, `threads`, `attachments`.

use gmail_mcp_core::error::Error;
use gmail_mcp_core::service::MailService;

/// The resource URI scheme.
pub const SCHEME: &str = "gmail";

/// A parsed resource target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceTarget {
    /// The mailbox list for an account.
    Mailboxes,
    /// A single mailbox by normalized name.
    Mailbox(String),
    /// A message by application ID.
    Message(String),
    /// A thread by application thread ID.
    Thread(String),
    /// Attachment metadata for a message.
    Attachments(String),
}

/// Parse a resource URI into an account alias and target.
pub fn parse_uri(uri: &str) -> Option<(String, ResourceTarget)> {
    let rest = uri.strip_prefix("gmail://")?;
    let mut parts = rest.splitn(3, '/');
    let account = parts.next()?.to_string();
    let kind = parts.next()?;
    let id = parts.next();
    let target = match (kind, id) {
        ("mailboxes", None) => ResourceTarget::Mailboxes,
        ("mailboxes", Some(name)) => ResourceTarget::Mailbox(name.to_string()),
        ("messages", Some(id)) => ResourceTarget::Message(id.to_string()),
        ("threads", Some(id)) => ResourceTarget::Thread(id.to_string()),
        ("attachments", Some(id)) => ResourceTarget::Attachments(id.to_string()),
        _ => return None,
    };
    Some((account, target))
}

/// Resolve a resource target against a service into JSON content.
pub async fn read_target(
    service: &dyn MailService,
    target: &ResourceTarget,
) -> Result<serde_json::Value, Error> {
    match target {
        ResourceTarget::Mailboxes => serde_json::to_value(service.list_mailboxes().await?)
            .map_err(|e| Error::Internal(format!("serialization failed: {e}"))),
        ResourceTarget::Mailbox(name) => serde_json::to_value(service.get_mailbox(name).await?)
            .map_err(|e| Error::Internal(format!("serialization failed: {e}"))),
        ResourceTarget::Message(id) => serde_json::to_value(service.get_message(id).await?)
            .map_err(|e| Error::Internal(format!("serialization failed: {e}"))),
        ResourceTarget::Thread(id) => serde_json::to_value(service.get_thread(id, false).await?)
            .map_err(|e| Error::Internal(format!("serialization failed: {e}"))),
        ResourceTarget::Attachments(id) => {
            serde_json::to_value(service.list_attachments(id).await?)
                .map_err(|e| Error::Internal(format!("serialization failed: {e}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_uris() {
        assert_eq!(
            parse_uri("gmail://personal/mailboxes"),
            Some(("personal".into(), ResourceTarget::Mailboxes))
        );
        assert_eq!(
            parse_uri("gmail://personal/mailboxes/inbox"),
            Some(("personal".into(), ResourceTarget::Mailbox("inbox".into())))
        );
        assert_eq!(
            parse_uri("gmail://personal/messages/m:1234"),
            Some(("personal".into(), ResourceTarget::Message("m:1234".into())))
        );
        assert_eq!(
            parse_uri("gmail://personal/threads/t:5678"),
            Some(("personal".into(), ResourceTarget::Thread("t:5678".into())))
        );
        assert_eq!(
            parse_uri("gmail://personal/attachments/a:999"),
            Some((
                "personal".into(),
                ResourceTarget::Attachments("a:999".into())
            ))
        );
        assert_eq!(parse_uri("http://other"), None);
        assert_eq!(parse_uri("gmail://personal/messages"), None);
    }
}
