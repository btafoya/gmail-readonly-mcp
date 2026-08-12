//! Human-readable terminal output for CLI commands.

use gmail_mcp_core::model::{
    AttachmentMeta, AttachmentResult, Header, Mailbox, MailboxStatus, MessageSummary,
};

pub fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("error: failed to serialize output: {e}"),
    }
}

pub fn print_mailboxes(mailboxes: &[Mailbox]) {
    for m in mailboxes {
        let attrs = if m.attributes.is_empty() {
            String::new()
        } else {
            format!(" [{}]", m.attributes.join(", "))
        };
        println!("{:<12} {}", m.name, m.raw_name);
        let _ = attrs;
    }
}

pub fn print_mailbox(m: &Mailbox) {
    println!("name:        {}", m.name);
    println!("raw name:    {}", m.raw_name);
    if !m.attributes.is_empty() {
        println!("attributes:  {}", m.attributes.join(", "));
    }
    if let Some(d) = &m.delimiter {
        println!("delimiter:   {d}");
    }
}

pub fn print_status(name: &str, s: &MailboxStatus) {
    println!("mailbox: {name}");
    println!("total:   {}", s.total);
    println!("unread:  {}", s.unread);
    println!("recent:  {}", s.recent);
}

pub fn print_search(results: &[MessageSummary]) {
    if results.is_empty() {
        println!("no messages found");
        return;
    }
    for r in results {
        let date = r
            .date
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".into());
        let sender = r
            .sender
            .as_ref()
            .map(|a| a.email.clone())
            .unwrap_or_else(|| "?".into());
        let subject = r.subject.as_deref().unwrap_or("(no subject)");
        println!("{}  {:<28} {}", date, sender, subject);
        println!("    id: {}", r.id);
    }
}

pub fn print_headers(headers: &[Header]) {
    for h in headers {
        println!("{}: {}", h.name, h.value);
    }
}

pub fn print_attachments(attachments: &[AttachmentMeta]) {
    if attachments.is_empty() {
        println!("no attachments");
        return;
    }
    for a in attachments {
        let kind = if a.is_inline { "inline" } else { "attachment" };
        println!(
            "{:<10} {:<12} {:>10}  {}",
            kind, a.content_type, a.size, a.filename
        );
        println!("    id: {}", a.id);
    }
}

pub fn print_attachment(result: &AttachmentResult) {
    let meta = result.meta();
    match result {
        AttachmentResult::Inline { data, .. } => {
            println!("filename:    {}", meta.filename);
            println!("content type: {}", meta.content_type);
            println!("size:        {} bytes", data.len());
            println!("inline:      true");
        }
        AttachmentResult::TempFile { path, .. } => {
            println!("filename:    {}", meta.filename);
            println!("content type: {}", meta.content_type);
            println!("size:        {} bytes", meta.size);
            println!("written to:  {}", path.display());
        }
    }
}
