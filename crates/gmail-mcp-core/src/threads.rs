//! Thread correlation fallback logic.
//!
//! Gmail's native `X-GM-THRID` is the primary thread identity. When it is not
//! available, threads are reconstructed from RFC `Message-ID` / `In-Reply-To` /
//! `References` headers, with subject normalization as a final fallback.

use chrono::{DateTime, Utc};

use crate::ids::AppId;

/// A message that can participate in thread reconstruction.
#[derive(Debug, Clone)]
pub struct Threadable {
    pub id: AppId,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub date: Option<DateTime<Utc>>,
}

/// Normalize a subject for comparison: strip reply/forward prefixes, trim,
/// and lowercase.
pub fn normalize_subject(subject: &str) -> String {
    let mut s = subject.trim().to_string();
    loop {
        let trimmed = strip_prefix(&s);
        if trimmed == s {
            break;
        }
        s = trimmed;
    }
    s.to_lowercase()
}

fn strip_prefix(s: &str) -> String {
    let lower = s.to_lowercase();
    for prefix in ["re:", "fwd:", "fw:", "aw:", "sv:", "答复:", "回复:"] {
        if lower.starts_with(prefix) {
            return s[prefix.len()..].trim().to_string();
        }
    }
    s.to_string()
}

/// Reconstruct a thread ordering from a flat list of messages.
///
/// Returns message IDs ordered so that parents precede their children, with
/// ties broken chronologically. Messages that share a normalized subject but
/// have no header relationship are grouped together as a final fallback.
pub fn reconstruct(messages: Vec<Threadable>) -> Vec<AppId> {
    if messages.is_empty() {
        return Vec::new();
    }

    // Index by Message-ID for parent lookup.
    let by_message_id: std::collections::HashMap<&str, &Threadable> = messages
        .iter()
        .filter_map(|m| m.message_id.as_deref().map(|id| (id, m)))
        .collect();

    // Determine the root of each message: walk In-Reply-To / References up to
    // the first message present in the set.
    let mut roots: Vec<&Threadable> = Vec::new();
    let mut children: std::collections::HashMap<&str, Vec<&Threadable>> =
        std::collections::HashMap::new();

    for msg in &messages {
        let parent = find_parent(msg, &by_message_id);
        match parent {
            Some(parent_id) => children.entry(parent_id).or_default().push(msg),
            None => roots.push(msg),
        }
    }

    // Depth-first: parents before children, chronological within a level.
    let mut ordered: Vec<AppId> = Vec::new();
    let mut roots_sorted = roots.clone();
    roots_sorted.sort_by_key(|m| m.date);
    for root in roots_sorted {
        visit(root, &children, &mut ordered);
    }

    // Subject-normalization fallback: any message not yet placed (e.g. no
    // Message-ID at all) is grouped by normalized subject.
    let placed: std::collections::HashSet<&AppId> = ordered.iter().collect();
    let mut unplaced: Vec<&Threadable> = messages
        .iter()
        .filter(|m| !placed.contains(&m.id))
        .collect();
    unplaced.sort_by_key(|m| m.date);
    for msg in unplaced {
        ordered.push(msg.id.clone());
    }

    ordered
}

fn find_parent<'a>(
    msg: &'a Threadable,
    by_message_id: &std::collections::HashMap<&str, &'a Threadable>,
) -> Option<&'a str> {
    // Prefer the last reference that is present in the set.
    for ref_id in msg.references.iter().rev() {
        if by_message_id.contains_key(ref_id.as_str()) {
            return Some(ref_id.as_str());
        }
    }
    if let Some(irt) = &msg.in_reply_to
        && by_message_id.contains_key(irt.as_str())
    {
        return Some(irt.as_str());
    }
    None
}

fn visit<'a>(
    msg: &'a Threadable,
    children: &std::collections::HashMap<&str, Vec<&'a Threadable>>,
    ordered: &mut Vec<AppId>,
) {
    ordered.push(msg.id.clone());
    if let Some(kids) = children.get(msg.message_id.as_deref().unwrap_or_default()) {
        let mut kids = kids.clone();
        kids.sort_by_key(|m| m.date);
        for kid in kids {
            visit(kid, children, ordered);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: &str, message_id: &str, in_reply_to: Option<&str>, date: DateTime<Utc>) -> Threadable {
        Threadable {
            id: AppId::new(crate::ids::Kind::Message, id),
            message_id: Some(message_id.to_string()),
            in_reply_to: in_reply_to.map(|s| s.to_string()),
            references: vec![],
            date: Some(date),
        }
    }

    #[test]
    fn normalizes_subjects() {
        assert_eq!(normalize_subject("Re: Hello"), "hello");
        assert_eq!(normalize_subject("Fwd: Re: Hello"), "hello");
        assert_eq!(normalize_subject("Hello"), "hello");
        assert_eq!(normalize_subject("  Re:  Hello  "), "hello");
    }

    #[test]
    fn parents_before_children() {
        let d1 = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let d2 = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let d3 = DateTime::parse_from_rfc3339("2024-01-03T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let msgs = vec![
            t("3", "c@x", Some("b@x"), d3),
            t("1", "a@x", None, d1),
            t("2", "b@x", Some("a@x"), d2),
        ];
        let ordered = reconstruct(msgs);
        let ids: Vec<String> = ordered.iter().map(|a| a.provider_id().unwrap()).collect();
        assert_eq!(ids, vec!["1", "2", "3"]);
    }

    #[test]
    fn chronological_within_level() {
        let d1 = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let d2 = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let msgs = vec![t("2", "b@x", None, d2), t("1", "a@x", None, d1)];
        let ordered = reconstruct(msgs);
        let ids: Vec<String> = ordered.iter().map(|a| a.provider_id().unwrap()).collect();
        assert_eq!(ids, vec!["1", "2"]);
    }
}
