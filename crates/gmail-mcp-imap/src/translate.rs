//! Translation of structured search filters into IMAP/Gmail search criteria.

use gmail_mcp_core::search::SearchRequest;

/// Build an IMAP `SEARCH` query string from a search request.
///
/// A Gmail-native `query` is passed through `X-GM-RAW`; structured filters
/// are translated to IMAP criteria and ANDed together.
pub fn to_imap_query(request: &SearchRequest) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(query) = &request.query
        && !query.trim().is_empty()
    {
        parts.push(format!("X-GM-RAW {}", quote(query)));
    }

    let f = &request.filters;
    if let Some(sender) = &f.sender {
        parts.push(format!("FROM {}", quote(sender)));
    }
    if let Some(recipients) = &f.recipients {
        parts.push(format!("TO {}", quote(recipients)));
    }
    if let Some(subject) = &f.subject {
        parts.push(format!("SUBJECT {}", quote(subject)));
    }
    if let Some(text) = &f.text {
        parts.push(format!("TEXT {}", quote(text)));
    }
    if let Some(from) = f.date_from {
        parts.push(format!("SINCE {}", imap_date(from)));
    }
    if let Some(to) = f.date_to {
        // IMAP BEFORE is exclusive; add a day to make the bound inclusive.
        let next = to + chrono::Duration::days(1);
        parts.push(format!("BEFORE {}", imap_date(next)));
    }
    if f.has_attachment == Some(true) {
        parts.push("X-GM-RAW \"has:attachment\"".to_string());
    }
    for flag in &f.flags {
        if let Some(criterion) = flag_criterion(flag) {
            parts.push(criterion.to_string());
        }
    }

    if parts.is_empty() {
        "ALL".to_string()
    } else {
        parts.join(" ")
    }
}

fn flag_criterion(flag: &str) -> Option<&'static str> {
    match flag.to_ascii_lowercase().as_str() {
        "seen" | "\\seen" => Some("SEEN"),
        "unseen" | "\\unseen" => Some("UNSEEN"),
        "flagged" | "\\flagged" => Some("FLAGGED"),
        "unflagged" | "\\unflagged" => Some("UNFLAGGED"),
        "answered" | "\\answered" => Some("ANSWERED"),
        "unanswered" | "\\unanswered" => Some("UNANSWERED"),
        "draft" | "\\draft" => Some("DRAFT"),
        "deleted" | "\\deleted" => Some("DELETED"),
        _ => None,
    }
}

fn imap_date(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%d-%b-%Y").to_string()
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gmail_mcp_core::search::SearchFilters;

    #[test]
    fn empty_is_all() {
        assert_eq!(to_imap_query(&SearchRequest::default()), "ALL");
    }

    #[test]
    fn gmail_query_uses_xgm_raw() {
        let req = SearchRequest {
            query: Some("from:alice has:attachment".into()),
            ..Default::default()
        };
        assert_eq!(
            to_imap_query(&req),
            "X-GM-RAW \"from:alice has:attachment\""
        );
    }

    #[test]
    fn structured_filters_and() {
        let req = SearchRequest {
            filters: SearchFilters {
                sender: Some("alice@example.com".into()),
                subject: Some("hello".into()),
                flags: vec!["unseen".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        let q = to_imap_query(&req);
        assert!(q.contains("FROM \"alice@example.com\""));
        assert!(q.contains("SUBJECT \"hello\""));
        assert!(q.contains("UNSEEN"));
    }

    #[test]
    fn date_bounds() {
        let from = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let to = chrono::DateTime::parse_from_rfc3339("2024-01-10T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let req = SearchRequest {
            filters: SearchFilters {
                date_from: Some(from),
                date_to: Some(to),
                ..Default::default()
            },
            ..Default::default()
        };
        let q = to_imap_query(&req);
        assert!(q.contains("SINCE 01-Jan-2024"));
        // BEFORE is exclusive, so the bound is shifted one day forward.
        assert!(q.contains("BEFORE 11-Jan-2024"));
    }

    #[test]
    fn quotes_are_escaped() {
        let req = SearchRequest {
            filters: SearchFilters {
                subject: Some("say \"hi\"".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(to_imap_query(&req).contains("SUBJECT \"say \\\"hi\\\"\""));
    }
}
