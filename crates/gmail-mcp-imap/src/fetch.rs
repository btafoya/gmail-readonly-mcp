//! Manual `UID FETCH` driver.
//!
//! async-imap parses `X-GM-THRID` but does not expose it through its `Fetch`
//! API. To honor the requirement that Gmail's native thread ID takes
//! precedence, we drive one `UID FETCH` ourselves and read the raw response,
//! capturing body, flags, and both Gmail IDs in a single round trip.

use async_imap::imap_proto::{AttributeValue, MessageSection, Response, SectionPath, Status};
use gmail_mcp_core::error::Error;

use crate::conn::ImapSession;

/// A message as returned by a `UID FETCH`.
#[derive(Debug, Default)]
pub struct FetchedMessage {
    pub seq: u32,
    pub uid: Option<u32>,
    pub size: Option<u32>,
    pub flags: Vec<String>,
    /// Full raw message (`BODY.PEEK[]`), when requested.
    pub body: Option<Vec<u8>>,
    /// Header block (`BODY.PEEK[HEADER]`), when requested.
    pub header: Option<Vec<u8>>,
    /// Partial text preview (`BODY.PEEK[TEXT]<0.N>`), when requested.
    pub text_preview: Option<Vec<u8>>,
    pub gmail_msgid: Option<u64>,
    pub gmail_thrid: Option<u64>,
    pub labels: Vec<String>,
    pub internal_date: Option<String>,
}

/// Run a `UID FETCH` and collect the parsed responses.
///
/// `query` is the raw FETCH data items, e.g. `(BODY.PEEK[] FLAGS X-GM-MSGID
/// X-GM-THRID)`. Uses `BODY.PEEK` so retrieval never sets `\Seen`.
pub async fn uid_fetch(
    session: &mut ImapSession,
    uid_set: &str,
    query: &str,
) -> Result<Vec<FetchedMessage>, Error> {
    let id = session
        .run_command(format!("UID FETCH {uid_set} {query}"))
        .await
        .map_err(|e| Error::Imap(e.to_string()))?;

    let mut out = Vec::new();
    loop {
        let Some(resp) = session
            .read_response()
            .await
            .map_err(|e| Error::Imap(e.to_string()))?
        else {
            break;
        };
        match resp.parsed() {
            Response::Fetch(seq, attrs) => out.push(parse_fetch(*seq, attrs)),
            Response::Done { status, tag, .. } if tag == &id => {
                if !matches!(status, Status::Ok) {
                    return Err(Error::Imap(format!("UID FETCH failed: {status:?}")));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_fetch(seq: u32, attrs: &[AttributeValue<'_>]) -> FetchedMessage {
    let mut msg = FetchedMessage {
        seq,
        ..Default::default()
    };
    for attr in attrs {
        match attr {
            AttributeValue::Uid(u) => msg.uid = Some(*u),
            AttributeValue::Rfc822Size(s) => msg.size = Some(*s),
            AttributeValue::Flags(flags) => {
                msg.flags = flags.iter().map(|f| f.to_string()).collect();
            }
            AttributeValue::BodySection {
                section: None,
                data: Some(body),
                ..
            } => msg.body = Some(body.to_vec()),
            AttributeValue::BodySection {
                section: Some(SectionPath::Full(MessageSection::Header)),
                data: Some(hdr),
                ..
            } => msg.header = Some(hdr.to_vec()),
            AttributeValue::BodySection {
                section: Some(SectionPath::Full(MessageSection::Text)),
                data: Some(text),
                ..
            } => msg.text_preview = Some(text.to_vec()),
            AttributeValue::Rfc822(Some(body)) => msg.body = Some(body.to_vec()),
            AttributeValue::Rfc822Header(Some(hdr)) => msg.header = Some(hdr.to_vec()),
            AttributeValue::GmailMsgId(id) => msg.gmail_msgid = Some(*id),
            AttributeValue::GmailThrId(id) => msg.gmail_thrid = Some(*id),
            AttributeValue::GmailLabels(labels) => {
                msg.labels = labels.iter().map(|l| l.to_string()).collect();
            }
            AttributeValue::InternalDate(d) => msg.internal_date = Some(d.to_string()),
            _ => {}
        }
    }
    msg
}
