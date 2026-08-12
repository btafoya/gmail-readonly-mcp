//! Mailbox name normalization between raw IMAP names and logical names.

use gmail_mcp_core::model::Mailbox;

/// Map a raw Gmail IMAP mailbox name to a normalized logical name.
///
/// Gmail's system mailboxes appear as `[Gmail]/...`; labels appear as plain
/// names. Unknown names are lowercased as a best effort.
pub fn normalize_name(raw: &str) -> String {
    match raw {
        "INBOX" => "inbox".to_string(),
        "[Gmail]/All Mail" => "all".to_string(),
        "[Gmail]/Sent Mail" => "sent".to_string(),
        "[Gmail]/Drafts" => "drafts".to_string(),
        "[Gmail]/Spam" => "spam".to_string(),
        "[Gmail]/Trash" => "trash".to_string(),
        "[Gmail]/Bin" => "trash".to_string(),
        "[Gmail]/Important" => "important".to_string(),
        "[Gmail]/Starred" => "starred".to_string(),
        _ => raw.to_lowercase(),
    }
}

/// Build a core `Mailbox` from a raw IMAP name and attributes.
pub fn to_mailbox(raw_name: &str, attributes: Vec<String>, delimiter: Option<String>) -> Mailbox {
    Mailbox {
        name: normalize_name(raw_name),
        raw_name: raw_name.to_string(),
        attributes,
        delimiter,
    }
}

/// Whether a normalized name refers to Spam or Trash.
pub fn is_spam_or_trash(name: &str) -> bool {
    matches!(name, "spam" | "trash")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_gmail_mailboxes() {
        assert_eq!(normalize_name("INBOX"), "inbox");
        assert_eq!(normalize_name("[Gmail]/All Mail"), "all");
        assert_eq!(normalize_name("[Gmail]/Sent Mail"), "sent");
        assert_eq!(normalize_name("[Gmail]/Spam"), "spam");
        assert_eq!(normalize_name("[Gmail]/Trash"), "trash");
        assert_eq!(normalize_name("[Gmail]/Bin"), "trash");
        assert_eq!(normalize_name("Work"), "work");
    }

    #[test]
    fn spam_trash_detection() {
        assert!(is_spam_or_trash("spam"));
        assert!(is_spam_or_trash("trash"));
        assert!(!is_spam_or_trash("inbox"));
    }
}
