//! MIME parsing: raw message bytes to a core `Message`.
//!
//! Uses `mailparse` for the MIME tree and the core `HtmlRenderer` for
//! sanitized HTML and Markdown representations.

use chrono::{DateTime, Utc};
use gmail_mcp_core::error::Error;
use gmail_mcp_core::ids::{AppId, Kind};
use gmail_mcp_core::model::{
    Address, AttachmentMeta, Flag, Header, Message, MimePart, ProviderIds,
};
use gmail_mcp_core::render::HtmlRenderer;
use mailparse::{DispositionType, MailHeader, ParsedMail};

/// Context needed to build a `Message` from raw bytes.
#[derive(Debug, Clone, Default)]
pub struct MessageContext {
    pub account: String,
    pub mailbox: String,
    pub provider: ProviderIds,
    pub flags: Vec<Flag>,
    pub labels: Vec<String>,
    pub size: u64,
}

/// Parse raw message bytes into a core `Message`.
pub fn parse_message(
    raw: &[u8],
    ctx: MessageContext,
    renderer: &dyn HtmlRenderer,
) -> Result<Message, Error> {
    let parsed = mailparse::parse_mail(raw)
        .map_err(|e| Error::Mime(format!("failed to parse message: {e}")))?;

    let headers = parsed
        .headers
        .iter()
        .map(|h| Header {
            name: h.get_key(),
            value: h.get_value(),
        })
        .collect::<Vec<_>>();

    let subject = header_value(&parsed.headers, "Subject");
    let date = header_value(&parsed.headers, "Date")
        .and_then(|d| DateTime::parse_from_rfc2822(&d).ok())
        .map(|d| d.with_timezone(&Utc));
    let sender = parse_addresses(&parsed.headers, "From").into_iter().next();
    let to = parse_addresses(&parsed.headers, "To");
    let cc = parse_addresses(&parsed.headers, "Cc");
    let bcc = parse_addresses(&parsed.headers, "Bcc");
    let reply_to = parse_addresses(&parsed.headers, "Reply-To");

    let provider = ctx.provider.clone();
    let msgid = provider
        .gmail_msgid
        .map(|m| m.to_string())
        .unwrap_or_else(|| provider.uid.map(|u| u.to_string()).unwrap_or_default());
    let thread_id = AppId::new(
        Kind::Thread,
        &provider
            .gmail_thrid
            .map(|t| t.to_string())
            .unwrap_or_else(|| msgid.clone()),
    );

    let mut walker = TreeWalker::new(&msgid);
    let mime_structure = walker.walk(&parsed, "1");

    let plain_text = walker.plain_text.take();
    let html = walker.html.take();
    let sanitized_html = html.as_deref().map(|h| renderer.sanitize(h));
    let markdown = sanitized_html
        .as_deref()
        .map(|h| renderer.to_markdown(h))
        .or_else(|| html.as_deref().map(|h| renderer.to_markdown(h)));
    let plain_text = plain_text.or_else(|| html.as_deref().map(|h| renderer.to_text(h)));

    Ok(Message {
        id: AppId::new(Kind::Message, &msgid),
        thread_id,
        account: ctx.account,
        mailbox: ctx.mailbox,
        provider,
        flags: ctx.flags,
        headers,
        sender,
        to,
        cc,
        bcc,
        reply_to,
        subject,
        date,
        size: ctx.size,
        labels: ctx.labels,
        mime_structure: Some(mime_structure),
        plain_text,
        html,
        sanitized_html,
        markdown,
        attachments: walker.attachments,
        raw_mime: Some(raw.to_vec()),
    })
}

/// Parse a header block (e.g. from `BODY.PEEK[HEADER]`) into headers.
pub fn parse_headers(raw: &[u8]) -> Result<Vec<Header>, Error> {
    let parsed = mailparse::parse_mail(raw)
        .map_err(|e| Error::Mime(format!("failed to parse headers: {e}")))?;
    Ok(parsed
        .headers
        .iter()
        .map(|h| Header {
            name: h.get_key(),
            value: h.get_value(),
        })
        .collect())
}

/// Find a MIME part by its dot-separated index path (e.g. `1.2`).
pub fn find_part<'a>(parsed: &'a ParsedMail<'a>, index: &str) -> Option<&'a ParsedMail<'a>> {
    let mut parts: Vec<&str> = index.split('.').collect();
    let first = parts.remove(0);
    if first != "1" {
        return None;
    }
    let mut current = parsed;
    for seg in parts {
        let idx: usize = seg.parse().ok()?;
        current = current.subparts.get(idx - 1)?;
    }
    Some(current)
}

struct TreeWalker<'a> {
    msgid: &'a str,
    plain_text: Option<String>,
    html: Option<String>,
    attachments: Vec<AttachmentMeta>,
}

impl<'a> TreeWalker<'a> {
    fn new(msgid: &'a str) -> Self {
        TreeWalker {
            msgid,
            plain_text: None,
            html: None,
            attachments: Vec::new(),
        }
    }

    fn walk(&mut self, part: &ParsedMail<'_>, index: &str) -> MimePart {
        let mimetype = part.ctype.mimetype.clone();
        let is_multipart = mimetype.starts_with("multipart/");
        let disposition = part.get_content_disposition();
        let filename = disposition
            .params
            .get("filename")
            .cloned()
            .or_else(|| part.ctype.params.get("name").cloned());
        let content_id = header_value(&part.headers, "Content-ID");

        let mut node = MimePart {
            index: index.to_string(),
            content_type: mimetype.clone(),
            charset: if part.ctype.charset.is_empty() {
                None
            } else {
                Some(part.ctype.charset.clone())
            },
            content_id,
            disposition: match disposition.disposition {
                DispositionType::Attachment => Some("attachment".to_string()),
                DispositionType::Inline => Some("inline".to_string()),
                _ => None,
            },
            filename: filename.clone(),
            is_attachment: false,
            size: part.raw_bytes.len(),
            children: Vec::new(),
        };

        if is_multipart {
            for (i, sub) in part.subparts.iter().enumerate() {
                let child_index = format!("{index}.{}", i + 1);
                let child = self.walk(sub, &child_index);
                node.children.push(child);
            }
        } else {
            let is_body_text = matches!(mimetype.as_str(), "text/plain" | "text/html")
                && filename.is_none()
                && disposition.disposition != DispositionType::Attachment;
            let is_attachment = !is_body_text
                && (disposition.disposition == DispositionType::Attachment
                    || filename.is_some()
                    || node.content_id.is_some());

            if is_attachment {
                let safe_name = filename.unwrap_or_else(|| "attachment".to_string());
                let meta = AttachmentMeta {
                    id: AppId::new(Kind::Attachment, &format!("{}.{}", self.msgid, index)),
                    filename: safe_name,
                    content_type: mimetype.clone(),
                    size: part.raw_bytes.len() as u64,
                    content_id: node.content_id.clone(),
                    is_inline: disposition.disposition == DispositionType::Inline
                        || node.content_id.is_some(),
                    part_index: index.to_string(),
                };
                self.attachments.push(meta);
                node.is_attachment = true;
            } else if mimetype == "text/plain" && self.plain_text.is_none() {
                self.plain_text = part.get_body().ok();
            } else if mimetype == "text/html" && self.html.is_none() {
                self.html = part.get_body().ok();
            }
        }

        node
    }
}

fn header_value(headers: &[MailHeader<'_>], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|h| h.get_key().eq_ignore_ascii_case(name))
        .map(|h| h.get_value())
        .filter(|v| !v.is_empty())
}

fn parse_addresses(headers: &[MailHeader<'_>], name: &str) -> Vec<Address> {
    let Some(header) = headers
        .iter()
        .find(|h| h.get_key().eq_ignore_ascii_case(name))
    else {
        return Vec::new();
    };
    mailparse::addrparse_header(header)
        .map(|addrs| {
            addrs
                .iter()
                .filter_map(|a| match a {
                    mailparse::MailAddr::Single(info) => Some(Address {
                        name: info.display_name.clone(),
                        email: info.addr.clone(),
                    }),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gmail_mcp_core::render::DefaultRenderer;

    const SIMPLE: &[u8] = b"From: Alice <alice@example.com>\r\nTo: bob@example.com\r\nSubject: Hello\r\nDate: Mon, 01 Jan 2024 10:00:00 +0000\r\nMessage-ID: <abc@example.com>\r\n\r\nHi Bob";
    const MULTIPART: &[u8] = b"From: alice@example.com\r\nSubject: With attachment\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"B\"\r\n\r\n--B\r\nContent-Type: text/plain\r\n\r\nBody text\r\n--B\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--B--\r\n";
    const ALTERNATIVE: &[u8] = b"From: alice@example.com\r\nSubject: Alt\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"A\"\r\n\r\n--A\r\nContent-Type: text/plain\r\n\r\nPlain body\r\n--A\r\nContent-Type: text/html\r\n\r\n<p>HTML body</p>\r\n--A--\r\n";

    fn ctx() -> MessageContext {
        MessageContext {
            account: "personal".into(),
            mailbox: "inbox".into(),
            provider: ProviderIds {
                gmail_msgid: Some(42),
                gmail_thrid: Some(7),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn parses_simple_message() {
        let renderer = DefaultRenderer::new();
        let msg = parse_message(SIMPLE, ctx(), &renderer).unwrap();
        assert_eq!(msg.subject.as_deref(), Some("Hello"));
        assert_eq!(msg.sender.as_ref().unwrap().email, "alice@example.com");
        assert_eq!(msg.plain_text.as_deref(), Some("Hi Bob"));
        assert!(msg.attachments.is_empty());
        assert_eq!(msg.id.provider_id().as_deref(), Some("42"));
        assert_eq!(msg.thread_id.provider_id().as_deref(), Some("7"));
    }

    #[test]
    fn parses_multipart_attachment() {
        let renderer = DefaultRenderer::new();
        let msg = parse_message(MULTIPART, ctx(), &renderer).unwrap();
        assert_eq!(msg.plain_text.as_deref(), Some("Body text"));
        assert_eq!(msg.attachments.len(), 1);
        let att = &msg.attachments[0];
        assert_eq!(att.filename, "report.pdf");
        assert_eq!(att.content_type, "application/pdf");
        assert_eq!(att.part_index, "1.2");
        assert!(!att.is_inline);
    }

    #[test]
    fn parses_alternative() {
        let renderer = DefaultRenderer::new();
        let msg = parse_message(ALTERNATIVE, ctx(), &renderer).unwrap();
        assert_eq!(msg.plain_text.as_deref(), Some("Plain body"));
        assert!(msg.html.as_deref().unwrap().contains("HTML body"));
        assert!(msg.sanitized_html.as_deref().unwrap().contains("HTML body"));
        assert!(msg.markdown.as_deref().unwrap().contains("HTML body"));
    }

    #[test]
    fn find_part_by_index() {
        let parsed = mailparse::parse_mail(MULTIPART).unwrap();
        let part = find_part(&parsed, "1.2").unwrap();
        assert_eq!(part.ctype.mimetype, "application/pdf");
        assert!(find_part(&parsed, "1.3").is_none());
        assert!(find_part(&parsed, "2").is_none());
    }

    #[test]
    fn parses_headers_block() {
        let headers = parse_headers(b"Subject: X\r\nFrom: a@b.c\r\n\r\n").unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, "Subject");
        assert_eq!(headers[0].value, "X");
    }
}
