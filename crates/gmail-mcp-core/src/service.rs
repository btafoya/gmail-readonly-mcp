//! The read-only mail service abstraction.
//!
//! This is the security boundary of the application: there are no mutation
//! methods here, and the concrete IMAP implementation must never expose any.
//! The CLI and MCP server are adapters over this trait.

use async_trait::async_trait;

use crate::error::Error;
use crate::model::AttachmentResult;
use crate::model::{
    AttachmentMeta, Header, Mailbox, MailboxStatus, Message, MessageSummary, Thread,
};
use crate::search::SearchRequest;

/// A read-only mail service bound to a single account.
///
/// Implementations must not mark messages `\Seen` and must never issue write
/// operations against the mailbox.
#[async_trait]
pub trait MailService: Send + Sync {
    /// Discover all mailboxes for the account.
    async fn list_mailboxes(&self) -> Result<Vec<Mailbox>, Error>;

    /// Get details for a normalized mailbox name.
    async fn get_mailbox(&self, name: &str) -> Result<Mailbox, Error>;

    /// Get status counts (total, unread, recent) for a normalized mailbox name.
    async fn get_mailbox_status(&self, name: &str) -> Result<MailboxStatus, Error>;

    /// Search messages. Results are metadata-first.
    async fn search_messages(&self, request: &SearchRequest) -> Result<Vec<MessageSummary>, Error>;

    /// Get a complete message by application ID.
    async fn get_message(&self, id: &str) -> Result<Message, Error>;

    /// Get message headers by application ID.
    async fn get_headers(&self, id: &str) -> Result<Vec<Header>, Error>;

    /// Get a thread by application thread ID.
    ///
    /// When `full` is true, entries include complete messages in chronological
    /// order; otherwise entries are summaries.
    async fn get_thread(&self, id: &str, full: bool) -> Result<Thread, Error>;

    /// List attachment metadata for a message.
    async fn list_attachments(&self, id: &str) -> Result<Vec<AttachmentMeta>, Error>;

    /// Retrieve an attachment by application ID.
    async fn get_attachment(&self, id: &str) -> Result<AttachmentResult, Error>;
}
