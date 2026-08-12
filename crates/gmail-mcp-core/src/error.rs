//! Stable application error categories for gmail-mcp.

use std::path::Path;

/// Application error categories. These are stable across the CLI, MCP, and
/// service layers so adapters can map them to their own error surfaces.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("account not found: {0} (available: {1})")]
    AccountNotFound(String, String),

    #[error("mailbox not found: {0}")]
    MailboxNotFound(String),

    #[error("message not found: {0}")]
    MessageNotFound(String),

    #[error("thread not found: {0}")]
    ThreadNotFound(String),

    #[error("attachment not found: {0}")]
    AttachmentNotFound(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("imap error: {0}")]
    Imap(String),

    #[error("mime error: {0}")]
    Mime(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Whether retrying the operation is likely to succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Network(_) | Error::Timeout(_) | Error::Imap(_) | Error::Auth(_)
        )
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Network(e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Config(format!("malformed TOML: {e}"))
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Error::Config(format!("failed to serialize TOML: {e}"))
    }
}

/// Convenience for building an `AccountNotFound` error with the available aliases.
pub fn account_not_found(alias: &str, available: &[String]) -> Error {
    Error::AccountNotFound(alias.to_string(), available.join(", "))
}

/// Convenience for a message-not-found error carrying the requested id.
pub fn message_not_found(id: &str) -> Error {
    Error::MessageNotFound(id.to_string())
}

/// Convenience for an attachment-not-found error carrying the requested id.
pub fn attachment_not_found(id: &str) -> Error {
    Error::AttachmentNotFound(id.to_string())
}

/// Convenience for a thread-not-found error carrying the requested id.
pub fn thread_not_found(id: &str) -> Error {
    Error::ThreadNotFound(id.to_string())
}

/// Convenience for a mailbox-not-found error carrying the requested name.
pub fn mailbox_not_found(name: &str) -> Error {
    Error::MailboxNotFound(name.to_string())
}

/// Convenience for an invalid-request error.
pub fn invalid_request(msg: impl Into<String>) -> Error {
    Error::InvalidRequest(msg.into())
}

/// Convenience for an internal error.
pub fn internal(msg: impl Into<String>) -> Error {
    Error::Internal(msg.into())
}

/// Convenience for a path-related internal error.
pub fn path_error(path: &Path, e: std::io::Error) -> Error {
    Error::Internal(format!("filesystem error on {}: {e}", path.display()))
}

/// Convenience for an IMAP-layer error.
pub fn imap_err(e: impl std::fmt::Display) -> Error {
    Error::Imap(e.to_string())
}
