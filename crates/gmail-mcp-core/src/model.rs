//! Domain models for gmail-mcp.
//!
//! These types are the shared language between the CLI, MCP server, and IMAP
//! implementation. They contain no IMAP- or MCP-specific details.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::AppId;

/// A single email address with an optional display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub name: Option<String>,
    pub email: String,
}

impl Address {
    pub fn new(name: Option<String>, email: String) -> Self {
        Address { name, email }
    }
}

/// A message header (name/value pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// An IMAP flag or keyword (e.g. `\Seen`, `\Flagged`, `$label1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flag(pub String);

impl Flag {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provider-side identifiers for a message.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderIds {
    /// Gmail `X-GM-MSGID`.
    pub gmail_msgid: Option<u64>,
    /// Gmail `X-GM-THRID`.
    pub gmail_thrid: Option<u64>,
    /// IMAP UID in the mailbox it was fetched from.
    pub uid: Option<u32>,
    /// IMAP sequence number in the mailbox it was fetched from.
    pub seq: Option<u32>,
    /// RFC `Message-ID` header value.
    pub message_id: Option<String>,
}

/// A mailbox (folder/label) as seen through IMAP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mailbox {
    /// Normalized logical name (e.g. `inbox`, `sent`, `all`, or a label name).
    pub name: String,
    /// Raw IMAP name (e.g. `INBOX`, `[Gmail]/Sent Mail`).
    pub raw_name: String,
    /// IMAP attributes (e.g. `\HasNoChildren`, `\Noselect`).
    pub attributes: Vec<String>,
    /// Hierarchy delimiter, if reported.
    pub delimiter: Option<String>,
}

/// Mailbox status counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxStatus {
    pub total: u32,
    pub unread: u32,
    pub recent: u32,
}

/// A node in the parsed MIME tree of a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MimePart {
    /// Part path used to address this part (e.g. `1`, `1.2`).
    pub index: String,
    /// MIME content type (e.g. `text/plain`, `multipart/alternative`).
    pub content_type: String,
    pub charset: Option<String>,
    /// `Content-ID` for inline/CID resources.
    pub content_id: Option<String>,
    /// `Content-Disposition` value.
    pub disposition: Option<String>,
    /// Original filename, if any.
    pub filename: Option<String>,
    /// Whether this part is an attachment (vs. inline body content).
    pub is_attachment: bool,
    /// Decoded size in bytes.
    pub size: usize,
    pub children: Vec<MimePart>,
}

/// Metadata for an attachment or inline resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentMeta {
    /// Application ID for retrieving this attachment.
    pub id: AppId,
    /// Sanitized filename.
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    /// `Content-ID` for inline resources.
    pub content_id: Option<String>,
    /// Whether this is an inline (CID-referenced) resource.
    pub is_inline: bool,
    /// MIME part path within the message.
    pub part_index: String,
}

/// A complete message.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: AppId,
    pub thread_id: AppId,
    pub account: String,
    pub mailbox: String,
    pub provider: ProviderIds,
    pub flags: Vec<Flag>,
    pub headers: Vec<Header>,
    pub sender: Option<Address>,
    pub to: Vec<Address>,
    pub cc: Vec<Address>,
    pub bcc: Vec<Address>,
    pub reply_to: Vec<Address>,
    pub subject: Option<String>,
    pub date: Option<DateTime<Utc>>,
    pub size: u64,
    /// Gmail labels (from `X-GM-LABELS`), when available.
    pub labels: Vec<String>,
    pub mime_structure: Option<MimePart>,
    pub plain_text: Option<String>,
    pub html: Option<String>,
    pub sanitized_html: Option<String>,
    pub markdown: Option<String>,
    pub attachments: Vec<AttachmentMeta>,
    /// Raw MIME bytes. Excluded from JSON serialization to keep it lean.
    #[serde(skip)]
    pub raw_mime: Option<Vec<u8>>,
}

/// Metadata-first summary of a message, used for search results.
#[derive(Debug, Clone, Serialize)]
pub struct MessageSummary {
    pub id: AppId,
    pub thread_id: AppId,
    pub account: String,
    pub mailbox: String,
    pub subject: Option<String>,
    pub sender: Option<Address>,
    pub date: Option<DateTime<Utc>>,
    pub flags: Vec<Flag>,
    pub snippet: Option<String>,
}

/// One entry in a thread.
#[derive(Debug, Clone, Serialize)]
pub struct ThreadEntry {
    pub id: AppId,
    pub subject: Option<String>,
    pub sender: Option<Address>,
    pub date: Option<DateTime<Utc>>,
    /// Short text summary (first line of the plain-text body).
    pub summary: Option<String>,
    /// Full message, present only for full thread retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

/// A conversation/thread.
#[derive(Debug, Clone, Serialize)]
pub struct Thread {
    pub id: AppId,
    pub subject: Option<String>,
    pub participants: Vec<Address>,
    pub message_count: usize,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// How thread identity was determined.
    pub correlation: String,
    /// Entries in chronological order (parents before children).
    pub messages: Vec<ThreadEntry>,
}

/// Result of an attachment retrieval.
#[derive(Debug, Clone)]
pub enum AttachmentResult {
    /// Content returned directly (at or below the direct-delivery threshold).
    Inline { data: Vec<u8>, meta: AttachmentMeta },
    /// Content written to a temporary file (above the threshold).
    TempFile {
        path: std::path::PathBuf,
        meta: AttachmentMeta,
    },
}

impl AttachmentResult {
    pub fn meta(&self) -> &AttachmentMeta {
        match self {
            AttachmentResult::Inline { meta, .. } => meta,
            AttachmentResult::TempFile { meta, .. } => meta,
        }
    }
}

/// A date range with inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_serializes_without_raw_mime() {
        let msg = Message {
            id: AppId::new(crate::ids::Kind::Message, "1"),
            thread_id: AppId::new(crate::ids::Kind::Thread, "1"),
            account: "personal".into(),
            mailbox: "inbox".into(),
            provider: ProviderIds::default(),
            flags: vec![],
            headers: vec![],
            sender: None,
            to: vec![],
            cc: vec![],
            bcc: vec![],
            reply_to: vec![],
            subject: Some("hi".into()),
            date: None,
            size: 0,
            labels: vec![],
            mime_structure: None,
            plain_text: None,
            html: None,
            sanitized_html: None,
            markdown: None,
            attachments: vec![],
            raw_mime: Some(vec![1, 2, 3]),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("raw_mime"));
        assert!(json.contains("\"subject\":\"hi\""));
    }
}
