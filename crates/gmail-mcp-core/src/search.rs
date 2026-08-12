//! Search request types.
//!
//! Search supports both Gmail-native query syntax and structured filters.
//! The concrete IMAP implementation translates these into IMAP/Gmail search
//! criteria.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Structured search filters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFilters {
    /// Mailbox to search. Defaults to the unified "All Mail" view.
    pub mailbox: Option<String>,
    pub sender: Option<String>,
    pub recipients: Option<String>,
    pub subject: Option<String>,
    pub text: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub has_attachment: Option<bool>,
    /// Flags to require (e.g. `\Seen`, `\Flagged`).
    pub flags: Vec<String>,
    /// Include Spam in results (excluded by default).
    pub include_spam: bool,
    /// Include Trash in results (excluded by default).
    pub include_trash: bool,
}

/// A search request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Gmail-native query syntax (e.g. `from:alice has:attachment`).
    pub query: Option<String>,
    pub filters: SearchFilters,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl SearchRequest {
    /// The mailbox to search, or the unified "all" view when unspecified.
    pub fn effective_mailbox(&self) -> String {
        self.filters
            .mailbox
            .clone()
            .unwrap_or_else(|| "all".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_mailbox_defaults_to_all() {
        let req = SearchRequest::default();
        assert_eq!(req.effective_mailbox(), "all");
        let req = SearchRequest {
            filters: SearchFilters {
                mailbox: Some("inbox".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(req.effective_mailbox(), "inbox");
    }
}
