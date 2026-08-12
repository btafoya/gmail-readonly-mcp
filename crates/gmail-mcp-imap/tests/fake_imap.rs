//! Fake IMAP server integration tests.
//!
//! A minimal in-process IMAP server exercises the full `ImapService` path
//! (connect, login, list, examine, search, fetch with Gmail extensions)
//! without any live Gmail. This validates the manual `UID FETCH` driver that
//! reads `X-GM-THRID`.

use std::sync::Arc;

use gmail_mcp_core::attachment::AttachmentPolicy;
use gmail_mcp_core::config::AccountConfig;
use gmail_mcp_core::render::DefaultRenderer;
use gmail_mcp_core::search::{SearchFilters, SearchRequest};
use gmail_mcp_core::service::MailService;
use gmail_mcp_imap::ImapService;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const MSG1: &[u8] = b"From: Alice <alice@example.com>\r\nTo: bob@example.com\r\nSubject: Hello\r\nDate: Mon, 01 Jan 2024 10:00:00 +0000\r\nMessage-ID: <a@x>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"B\"\r\n\r\n--B\r\nContent-Type: text/plain\r\n\r\nHi Bob, see attached.\r\n--B\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--B--\r\n";
const MSG2: &[u8] = b"From: Bob <bob@example.com>\r\nTo: alice@example.com\r\nSubject: Re: Hello\r\nDate: Tue, 02 Jan 2024 10:00:00 +0000\r\nMessage-ID: <b@x>\r\nIn-Reply-To: <a@x>\r\n\r\nGot it.";

async fn respond<W: AsyncWrite + Unpin>(writer: &mut W, tag: &str, text: &str) {
    writer
        .write_all(format!("{tag} {text}\r\n").as_bytes())
        .await
        .unwrap();
}

async fn handle(mut stream: TcpStream) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    writer.write_all(b"* OK fake imap ready\r\n").await.unwrap();
    writer.flush().await.unwrap();

    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await.unwrap() == 0 {
            break;
        }
        let trimmed = line.trim_end();
        let (tag, rest) = trimmed.split_once(' ').unwrap_or(("", trimmed));
        let upper = rest.to_uppercase();

        if upper.starts_with("LOGIN") {
            respond(&mut writer, tag, "OK LOGIN completed").await;
        } else if upper.starts_with("LIST") {
            writer
                .write_all(b"* LIST (\\HasNoChildren) \"/\" INBOX\r\n")
                .await
                .unwrap();
            writer
                .write_all(b"* LIST (\\HasNoChildren) \"/\" \"[Gmail]/All Mail\"\r\n")
                .await
                .unwrap();
            writer
                .write_all(b"* LIST (\\HasNoChildren) \"/\" \"[Gmail]/Sent Mail\"\r\n")
                .await
                .unwrap();
            respond(&mut writer, tag, "OK LIST completed").await;
        } else if upper.starts_with("EXAMINE") {
            writer
                .write_all(b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n")
                .await
                .unwrap();
            writer.write_all(b"* 2 EXISTS\r\n").await.unwrap();
            writer.write_all(b"* 0 RECENT\r\n").await.unwrap();
            respond(&mut writer, tag, "OK [READ-ONLY] EXAMINE completed").await;
        } else if upper.starts_with("UID SEARCH") {
            let uids = if upper.contains("X-GM-THRID 7") {
                "1 2"
            } else {
                "1"
            };
            writer
                .write_all(format!("* SEARCH {uids}\r\n").as_bytes())
                .await
                .unwrap();
            respond(&mut writer, tag, "OK SEARCH completed").await;
        } else if upper.starts_with("UID FETCH") {
            let uid_set = rest.split_whitespace().nth(2).unwrap_or("1").to_string();
            let full = upper.contains("BODY.PEEK[]");
            for uid in uid_set.split(',') {
                let (msg, msgid, thrid) = match uid {
                    "2" => (MSG2, 43u64, 7u64),
                    _ => (MSG1, 42u64, 7u64),
                };
                if full {
                    write_full_fetch(&mut writer, uid, msg, msgid, thrid).await;
                } else {
                    write_header_fetch(&mut writer, uid, msg, msgid, thrid).await;
                }
            }
            respond(&mut writer, tag, "OK FETCH completed").await;
        } else if upper.starts_with("STATUS") {
            writer
                .write_all(b"* STATUS \"[Gmail]/All Mail\" (MESSAGES 2 UNSEEN 1 RECENT 0)\r\n")
                .await
                .unwrap();
            respond(&mut writer, tag, "OK STATUS completed").await;
        } else if upper.starts_with("LOGOUT") {
            writer.write_all(b"* BYE logging out\r\n").await.unwrap();
            respond(&mut writer, tag, "OK LOGOUT completed").await;
            break;
        } else {
            respond(&mut writer, tag, "OK").await;
        }
    }
}

async fn write_full_fetch<W: AsyncWrite + Unpin>(
    writer: &mut W,
    uid: &str,
    msg: &[u8],
    msgid: u64,
    thrid: u64,
) {
    let flags = if uid == "1" { "(\\Seen)" } else { "()" };
    let labels = if uid == "1" {
        " X-GM-LABELS (\"\\\\Inbox\")"
    } else {
        ""
    };
    writer
        .write_all(
            format!(
                "* {uid} FETCH (UID {uid} FLAGS {flags}{labels} X-GM-MSGID {msgid} X-GM-THRID {thrid} BODY[] {{{}}}\r\n",
                msg.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    writer.write_all(msg).await.unwrap();
    writer.write_all(b")\r\n").await.unwrap();
}

async fn write_header_fetch<W: AsyncWrite + Unpin>(
    writer: &mut W,
    uid: &str,
    msg: &[u8],
    msgid: u64,
    thrid: u64,
) {
    // Split the raw message into header block and body at the first blank line.
    let (header, body) = match msg.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(i) => (&msg[..i + 2], &msg[i + 4..]),
        None => (msg, &[][..]),
    };
    let preview = &body[..body.len().min(50)];
    writer
        .write_all(
            format!(
                "* {uid} FETCH (UID {uid} FLAGS () X-GM-MSGID {msgid} X-GM-THRID {thrid} BODY[HEADER] {{{}}}\r\n",
                header.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    writer.write_all(header).await.unwrap();
    writer
        .write_all(format!(" BODY[TEXT]<0> {{{}}}\r\n", preview.len()).as_bytes())
        .await
        .unwrap();
    writer.write_all(preview).await.unwrap();
    writer.write_all(b")\r\n").await.unwrap();
}

fn account(port: u16) -> AccountConfig {
    AccountConfig {
        email: "me@gmail.com".into(),
        app_password: "abcd efgh ijkl mnop".into(),
        imap_host: "127.0.0.1".into(),
        imap_port: port,
        tls: false,
        timezone: None,
        default: true,
        connect_timeout_secs: Some(5),
    }
}

fn service(port: u16) -> Arc<ImapService> {
    Arc::new(ImapService::new(
        "personal".into(),
        account(port),
        AttachmentPolicy::default(),
        Arc::new(DefaultRenderer::new()),
    ))
}

#[tokio::test]
async fn full_service_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(handle(stream));
        }
    });

    let svc = service(port);

    // Mailbox discovery + normalization.
    let mailboxes = svc.list_mailboxes().await.unwrap();
    let names: Vec<&str> = mailboxes.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"inbox"));
    assert!(names.contains(&"all"));
    assert!(names.contains(&"sent"));

    // Mailbox status.
    let status = svc.get_mailbox_status("all").await.unwrap();
    assert_eq!(status.total, 2);
    assert_eq!(status.unread, 1);

    // Search (metadata-first).
    let results = svc
        .search_messages(&SearchRequest {
            query: Some("from:alice".into()),
            filters: SearchFilters::default(),
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].subject.as_deref(), Some("Hello"));
    assert_eq!(
        results[0].sender.as_ref().unwrap().email,
        "alice@example.com"
    );
    assert_eq!(results[0].id.provider_id().as_deref(), Some("42"));

    // Full message retrieval (validates the manual UID FETCH path).
    let msg = svc.get_message(&results[0].id.to_string()).await.unwrap();
    assert_eq!(msg.subject.as_deref(), Some("Hello"));
    assert_eq!(msg.plain_text.as_deref(), Some("Hi Bob, see attached."));
    assert_eq!(msg.thread_id.provider_id().as_deref(), Some("7"));
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].filename, "report.pdf");

    // Headers.
    let headers = svc.get_headers(&results[0].id.to_string()).await.unwrap();
    assert!(
        headers
            .iter()
            .any(|h| h.name == "Subject" && h.value == "Hello")
    );

    // Attachments.
    let attachments = svc
        .list_attachments(&results[0].id.to_string())
        .await
        .unwrap();
    assert_eq!(attachments.len(), 1);
    let att = svc
        .get_attachment(&attachments[0].id.to_string())
        .await
        .unwrap();
    match att {
        gmail_mcp_core::model::AttachmentResult::Inline { data, .. } => {
            // Base64 transfer encoding is decoded: "JVBERi0xLjQK" -> "%PDF-1.4\n".
            assert_eq!(data, b"%PDF-1.4\n");
        }
        _ => panic!("expected inline attachment"),
    }

    // Thread retrieval (X-GM-THRID 7 -> messages 1 and 2).
    let thread = svc
        .get_thread(&msg.thread_id.to_string(), false)
        .await
        .unwrap();
    assert_eq!(thread.message_count, 2);
    assert_eq!(thread.correlation, "gmail-thrid");
    assert_eq!(thread.messages.len(), 2);

    server.abort();
}
