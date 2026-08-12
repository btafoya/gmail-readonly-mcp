//! Message rendering abstractions.
//!
//! Third-party HTML sanitization / HTML-to-Markdown / HTML-to-text libraries
//! are isolated behind the [`HtmlRenderer`] trait so they remain replaceable
//! without leaking into the domain API.

use crate::error::Error;
use crate::model::{Message, Thread};

/// A requested output representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    Json,
    Markdown,
    Html,
    Text,
    Raw,
}

impl RenderFormat {
    /// Parse a format name (as used by CLI flags and MCP arguments).
    pub fn parse(s: &str) -> Option<RenderFormat> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Some(RenderFormat::Json),
            "markdown" | "md" => Some(RenderFormat::Markdown),
            "html" => Some(RenderFormat::Html),
            "text" | "plain" => Some(RenderFormat::Text),
            "raw" => Some(RenderFormat::Raw),
            _ => None,
        }
    }
}

/// HTML processing abstraction.
pub trait HtmlRenderer: Send + Sync {
    /// Remove active/executable content (scripts, forms, event handlers,
    /// dangerous URL schemes) while retaining useful formatting.
    fn sanitize(&self, html: &str) -> String;
    /// Convert HTML to Markdown.
    fn to_markdown(&self, html: &str) -> String;
    /// Convert HTML to plain text.
    fn to_text(&self, html: &str) -> String;
}

/// Message/thread rendering abstraction.
pub trait MessageRenderer: Send + Sync {
    fn render_message(&self, msg: &Message, format: RenderFormat) -> Result<String, Error>;
    fn render_thread(&self, thread: &Thread, format: RenderFormat) -> Result<String, Error>;
}

/// Default renderer backed by ammonia, html2md, and html2text.
pub struct DefaultRenderer {
    html: Box<dyn HtmlRenderer>,
}

impl DefaultRenderer {
    pub fn new() -> Self {
        DefaultRenderer {
            html: Box::new(DefaultHtmlRenderer),
        }
    }
}

impl Default for DefaultRenderer {
    fn default() -> Self {
        Self::new()
    }
}

struct DefaultHtmlRenderer;

impl HtmlRenderer for DefaultHtmlRenderer {
    fn sanitize(&self, html: &str) -> String {
        ammonia::clean(html)
    }

    fn to_markdown(&self, html: &str) -> String {
        html2md::parse_html(html)
    }

    fn to_text(&self, html: &str) -> String {
        html2text::from_read(html.as_bytes(), 100).unwrap_or_default()
    }
}

impl HtmlRenderer for DefaultRenderer {
    fn sanitize(&self, html: &str) -> String {
        self.html.sanitize(html)
    }

    fn to_markdown(&self, html: &str) -> String {
        self.html.to_markdown(html)
    }

    fn to_text(&self, html: &str) -> String {
        self.html.to_text(html)
    }
}

impl MessageRenderer for DefaultRenderer {
    fn render_message(&self, msg: &Message, format: RenderFormat) -> Result<String, Error> {
        match format {
            RenderFormat::Json => serde_json::to_string_pretty(msg)
                .map_err(|e| Error::Internal(format!("failed to serialize message: {e}"))),
            RenderFormat::Markdown => Ok(render_markdown(msg, &*self.html)),
            RenderFormat::Html => {
                let html = msg
                    .sanitized_html
                    .clone()
                    .or_else(|| msg.html.clone())
                    .unwrap_or_default();
                Ok(html)
            }
            RenderFormat::Text => {
                let text = msg
                    .plain_text
                    .clone()
                    .or_else(|| msg.html.as_deref().map(|h| self.html.to_text(h)))
                    .unwrap_or_default();
                Ok(text)
            }
            RenderFormat::Raw => msg
                .raw_mime
                .as_ref()
                .map(|raw| String::from_utf8_lossy(raw).into_owned())
                .ok_or_else(|| {
                    Error::InvalidRequest("raw MIME not available for this message".into())
                }),
        }
    }

    fn render_thread(&self, thread: &Thread, format: RenderFormat) -> Result<String, Error> {
        match format {
            RenderFormat::Json => serde_json::to_string_pretty(thread)
                .map_err(|e| Error::Internal(format!("failed to serialize thread: {e}"))),
            RenderFormat::Markdown => Ok(render_thread_markdown(thread)),
            RenderFormat::Html | RenderFormat::Text | RenderFormat::Raw => {
                // Threads are structured; fall back to a readable text form.
                Ok(render_thread_text(thread))
            }
        }
    }
}

fn render_markdown(msg: &Message, renderer: &dyn HtmlRenderer) -> String {
    let mut out = String::new();
    if let Some(subject) = &msg.subject {
        out.push_str(&format!("# {subject}\n\n"));
    }
    if let Some(sender) = &msg.sender {
        out.push_str(&format!("**From:** {}\n\n", format_address(sender)));
    }
    if !msg.to.is_empty() {
        out.push_str(&format!("**To:** {}\n\n", format_addresses(&msg.to)));
    }
    if let Some(date) = msg.date {
        out.push_str(&format!("**Date:** {date}\n\n"));
    }
    if let Some(md) = &msg.markdown {
        out.push_str(md);
    } else if let Some(html) = &msg.html {
        out.push_str(&renderer.to_markdown(html));
    } else if let Some(text) = &msg.plain_text {
        out.push_str(text);
    }
    if !msg.attachments.is_empty() {
        out.push_str("\n\n**Attachments:**\n");
        for a in &msg.attachments {
            out.push_str(&format!("- {} ({})\n", a.filename, a.content_type));
        }
    }
    out
}

fn render_thread_markdown(thread: &Thread) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# {}\n\n",
        thread.subject.as_deref().unwrap_or("(no subject)")
    ));
    out.push_str(&format!(
        "**{} messages** · {} participants\n\n",
        thread.message_count,
        thread.participants.len()
    ));
    for entry in &thread.messages {
        out.push_str(&format!(
            "- **{}** ({}) — {}\n",
            entry
                .sender
                .as_ref()
                .map(format_address)
                .unwrap_or_else(|| "?".into()),
            entry
                .date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "?".into()),
            entry.summary.as_deref().unwrap_or("")
        ));
    }
    out
}

fn render_thread_text(thread: &Thread) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Thread: {}\n{} messages, {} participants\n",
        thread.subject.as_deref().unwrap_or("(no subject)"),
        thread.message_count,
        thread.participants.len()
    ));
    for entry in &thread.messages {
        out.push_str(&format!(
            "  {} ({}) — {}\n",
            entry
                .sender
                .as_ref()
                .map(format_address)
                .unwrap_or_else(|| "?".into()),
            entry
                .date
                .map(|d| d.to_string())
                .unwrap_or_else(|| "?".into()),
            entry.summary.as_deref().unwrap_or("")
        ));
    }
    out
}

fn format_address(a: &crate::model::Address) -> String {
    match &a.name {
        Some(name) if !name.is_empty() => format!("{name} <{}>", a.email),
        _ => a.email.clone(),
    }
}

fn format_addresses(list: &[crate::model::Address]) -> String {
    list.iter()
        .map(format_address)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AppId, Kind};
    use crate::model::{Address, Message, ProviderIds, Thread, ThreadEntry};

    fn sample_message() -> Message {
        Message {
            id: AppId::new(Kind::Message, "1"),
            thread_id: AppId::new(Kind::Thread, "1"),
            account: "personal".into(),
            mailbox: "inbox".into(),
            provider: ProviderIds::default(),
            flags: vec![],
            headers: vec![],
            sender: Some(Address::new(
                Some("Alice".into()),
                "alice@example.com".into(),
            )),
            to: vec![Address::new(None, "bob@example.com".into())],
            cc: vec![],
            bcc: vec![],
            reply_to: vec![],
            subject: Some("Hello".into()),
            date: None,
            size: 0,
            labels: vec![],
            mime_structure: None,
            plain_text: Some("Hi Bob".into()),
            html: Some("<p>Hi <b>Bob</b></p>".into()),
            sanitized_html: Some("<p>Hi <b>Bob</b></p>".into()),
            markdown: Some("Hi **Bob**".into()),
            attachments: vec![],
            raw_mime: Some(b"Subject: Hello\r\n\r\nHi Bob".to_vec()),
        }
    }

    #[test]
    fn sanitize_removes_scripts() {
        let r = DefaultRenderer::new();
        let out = r
            .html
            .sanitize("<p>ok</p><script>alert(1)</script><form>x</form>");
        assert!(!out.contains("<script"));
        assert!(!out.contains("<form"));
        assert!(out.contains("ok"));
    }

    #[test]
    fn markdown_from_html() {
        let r = DefaultRenderer::new();
        assert_eq!(
            r.html.to_markdown("<p>Hi <b>Bob</b></p>").trim(),
            "Hi **Bob**"
        );
    }

    #[test]
    fn text_from_html() {
        let r = DefaultRenderer::new();
        assert!(r.html.to_text("<p>Hi Bob</p>").contains("Hi Bob"));
    }

    #[test]
    fn renders_all_formats() {
        let r = DefaultRenderer::new();
        let msg = sample_message();
        assert!(
            r.render_message(&msg, RenderFormat::Json)
                .unwrap()
                .contains("\"subject\"")
        );
        assert!(
            r.render_message(&msg, RenderFormat::Markdown)
                .unwrap()
                .contains("Hello")
        );
        assert!(
            r.render_message(&msg, RenderFormat::Html)
                .unwrap()
                .contains("<p>")
        );
        assert!(
            r.render_message(&msg, RenderFormat::Text)
                .unwrap()
                .contains("Hi Bob")
        );
        assert!(
            r.render_message(&msg, RenderFormat::Raw)
                .unwrap()
                .contains("Subject: Hello")
        );
    }

    #[test]
    fn renders_thread() {
        let r = DefaultRenderer::new();
        let thread = Thread {
            id: AppId::new(Kind::Thread, "1"),
            subject: Some("Hello".into()),
            participants: vec![Address::new(None, "a@example.com".into())],
            message_count: 1,
            date_range: None,
            correlation: "gmail".into(),
            messages: vec![ThreadEntry {
                id: AppId::new(Kind::Message, "1"),
                subject: Some("Hello".into()),
                sender: Some(Address::new(None, "a@example.com".into())),
                date: None,
                summary: Some("Hi".into()),
                message: None,
            }],
        };
        assert!(
            r.render_thread(&thread, RenderFormat::Json)
                .unwrap()
                .contains("Hello")
        );
        assert!(
            r.render_thread(&thread, RenderFormat::Markdown)
                .unwrap()
                .contains("1 messages")
        );
    }
}
