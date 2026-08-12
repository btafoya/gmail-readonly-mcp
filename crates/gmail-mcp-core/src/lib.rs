//! gmail-mcp-core: domain models, configuration, and the read-only mail
//! service abstraction shared by the CLI and MCP server.
//!
//! This crate contains no IMAP- or MCP-specific implementation details.

pub mod attachment;
pub mod cache;
pub mod config;
pub mod error;
pub mod ids;
pub mod model;
pub mod render;
pub mod search;
pub mod service;
pub mod threads;

pub use attachment::AttachmentPolicy;
pub use config::{AccountConfig, ConfigFile};
pub use error::Error;
pub use ids::{AppId, Kind};
pub use model::{
    Address, AttachmentMeta, AttachmentResult, Flag, Header, Mailbox, MailboxStatus, Message,
    MessageSummary, MimePart, ProviderIds, Thread, ThreadEntry,
};
pub use render::{DefaultRenderer, HtmlRenderer, MessageRenderer, RenderFormat};
pub use search::{SearchFilters, SearchRequest};
pub use service::MailService;
