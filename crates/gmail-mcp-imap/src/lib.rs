//! gmail-mcp-imap: the concrete IMAP implementation of the core read-only
//! mail service.
//!
//! The IMAP dependency is isolated behind the core `MailService` trait. This
//! crate never issues write operations and never marks messages `\Seen`.

pub mod conn;
pub mod fetch;
pub mod mailbox;
pub mod mime;
pub mod service;
pub mod translate;

pub use service::ImapService;
