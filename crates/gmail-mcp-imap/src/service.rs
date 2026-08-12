//! The concrete IMAP implementation of the core `MailService`.
//!
//! Read-only by construction: mailboxes are opened with `EXAMINE` and message
//! bodies are fetched with `BODY.PEEK`, so retrieval never sets `\Seen` and no
//! write command is ever issued.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::TryStreamExt;
use gmail_mcp_core::attachment::AttachmentPolicy;
use gmail_mcp_core::config::AccountConfig;
use gmail_mcp_core::error::{
    Error, attachment_not_found, imap_err, invalid_request, mailbox_not_found, message_not_found,
    thread_not_found,
};
use gmail_mcp_core::ids::{AppId, Kind};
use gmail_mcp_core::model::{
    Address, AttachmentMeta, Flag, Mailbox, MailboxStatus, Message, MessageSummary, ProviderIds,
    Thread, ThreadEntry,
};
use gmail_mcp_core::render::HtmlRenderer;
use gmail_mcp_core::search::SearchRequest;
use gmail_mcp_core::service::MailService;

use crate::conn::Connection;
use crate::fetch::{self, FetchedMessage};
use crate::mailbox::{normalize_name, to_mailbox};
use crate::mime::{self, MessageContext};
use crate::translate;

/// The mailbox list cache TTL.
const MAILBOX_CACHE_TTL: Duration = Duration::from_secs(60);

/// A read-only IMAP mail service bound to one account.
pub struct ImapService {
    alias: String,
    policy: AttachmentPolicy,
    renderer: Arc<dyn HtmlRenderer>,
    conn: Connection,
    mailboxes: Mutex<Option<(Instant, Vec<Mailbox>)>>,
}

impl ImapService {
    pub fn new(
        alias: String,
        account: AccountConfig,
        policy: AttachmentPolicy,
        renderer: Arc<dyn HtmlRenderer>,
    ) -> Self {
        ImapService {
            alias,
            policy,
            renderer,
            conn: Connection::new(account),
            mailboxes: Mutex::new(None),
        }
    }

    /// Drop the underlying connection (used on shutdown).
    pub fn shutdown(&self) {
        self.conn.drop_session();
    }

    async fn list_mailboxes_cached(&self) -> Result<Vec<Mailbox>, Error> {
        {
            let cached = self.mailboxes.lock().unwrap();
            if let Some((at, list)) = cached.as_ref()
                && at.elapsed() < MAILBOX_CACHE_TTL
            {
                return Ok(list.clone());
            }
        }
        let list = self
            .conn
            .run(|s| {
                Box::pin(async move {
                    let mut stream = s.list(None, Some("*")).await.map_err(imap_err)?;
                    let mut out = Vec::new();
                    while let Some(name) = stream.try_next().await.map_err(imap_err)? {
                        out.push(to_mailbox(
                            name.name(),
                            name.attributes().iter().map(|a| format!("{a:?}")).collect(),
                            name.delimiter().map(|d| d.to_string()),
                        ));
                    }
                    Ok(out)
                })
            })
            .await?;
        *self.mailboxes.lock().unwrap() = Some((Instant::now(), list.clone()));
        Ok(list)
    }

    /// Resolve a normalized mailbox name to its raw IMAP name.
    async fn resolve_raw(&self, name: &str) -> Result<String, Error> {
        let mailboxes = self.list_mailboxes_cached().await?;
        mailboxes
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.raw_name.clone())
            .ok_or_else(|| mailbox_not_found(name))
    }

    /// The mailboxes to search, honoring spam/trash inclusion.
    async fn search_mailboxes(&self, request: &SearchRequest) -> Result<Vec<String>, Error> {
        if let Some(mailbox) = &request.filters.mailbox {
            return Ok(vec![self.resolve_raw(mailbox).await?]);
        }
        let mut names = vec!["all".to_string()];
        if request.filters.include_spam {
            names.push("spam".to_string());
        }
        if request.filters.include_trash {
            names.push("trash".to_string());
        }
        let mut raw = Vec::new();
        for name in names {
            if let Ok(r) = self.resolve_raw(&name).await {
                raw.push(r);
            }
        }
        Ok(raw)
    }

    async fn search_in_mailbox(
        &self,
        raw: &str,
        query: &str,
        offset: usize,
        limit: Option<usize>,
    ) -> Result<Vec<MessageSummary>, Error> {
        let alias = self.alias.clone();
        self.conn
            .run(|s| {
                let raw = raw.to_string();
                let query = query.to_string();
                let alias = alias.clone();
                Box::pin(async move {
                    s.examine(&raw).await.map_err(|e| match e {
                        async_imap::error::Error::No(m) => mailbox_not_found(&m),
                        other => Error::Imap(other.to_string()),
                    })?;
                    let uids = s.uid_search(&query).await.map_err(imap_err)?;
                    let mut uids: Vec<u32> = uids.into_iter().collect();
                    uids.sort_unstable();
                    let uids = apply_paging(uids, offset, limit);
                    if uids.is_empty() {
                        return Ok(Vec::new());
                    }
                    let uid_set = join_uids(&uids);
                    let fetched = fetch::uid_fetch(
                        s,
                        &uid_set,
                        "(BODY.PEEK[HEADER] BODY.PEEK[TEXT]<0.300> FLAGS X-GM-MSGID X-GM-THRID)",
                    )
                    .await?;
                    Ok(fetched
                        .into_iter()
                        .map(|f| summary_from_fetch(&alias, f, &raw))
                        .collect())
                })
            })
            .await
    }

    async fn fetch_message_by_id(&self, id: &str) -> Result<Message, Error> {
        let id = id.to_string();
        let app = AppId::from(id.as_str());
        let provider_id = app
            .provider_id()
            .ok_or_else(|| invalid_request(format!("malformed message id `{id}`")))?;
        let raw = self.resolve_raw("all").await?;
        let alias = self.alias.clone();
        let renderer = self.renderer.clone();
        self.conn
            .run(|s| {
                let id = id.clone();
                let raw = raw.clone();
                let provider_id = provider_id.clone();
                let alias = alias.clone();
                let renderer = renderer.clone();
                Box::pin(async move {
                    s.examine(&raw).await.map_err(|e| match e {
                        async_imap::error::Error::No(m) => mailbox_not_found(&m),
                        other => Error::Imap(other.to_string()),
                    })?;
                    let uids = s
                        .uid_search(format!("X-GM-MSGID {provider_id}"))
                        .await
                        .map_err(imap_err)?;
                    let uid = uids
                        .into_iter()
                        .next()
                        .ok_or_else(|| message_not_found(&id))?;
                    let fetched = fetch::uid_fetch(
                        s,
                        &uid.to_string(),
                        "(BODY.PEEK[] FLAGS X-GM-MSGID X-GM-THRID X-GM-LABELS)",
                    )
                    .await?;
                    let f = fetched
                        .into_iter()
                        .next()
                        .ok_or_else(|| message_not_found(&id))?;
                    let body = f
                        .body
                        .ok_or_else(|| Error::Mime("fetch returned no body".into()))?;
                    let ctx = MessageContext {
                        account: alias,
                        mailbox: normalize_name(&raw),
                        provider: ProviderIds {
                            gmail_msgid: f.gmail_msgid,
                            gmail_thrid: f.gmail_thrid,
                            uid: f.uid,
                            seq: Some(f.seq),
                            message_id: None,
                        },
                        flags: f.flags.into_iter().map(Flag).collect(),
                        labels: f.labels,
                        size: f.size.map(|s| s as u64).unwrap_or(body.len() as u64),
                    };
                    mime::parse_message(&body, ctx, &*renderer)
                })
            })
            .await
    }
}

#[async_trait]
impl MailService for ImapService {
    async fn list_mailboxes(&self) -> Result<Vec<Mailbox>, Error> {
        self.list_mailboxes_cached().await
    }

    async fn get_mailbox(&self, name: &str) -> Result<Mailbox, Error> {
        let mailboxes = self.list_mailboxes_cached().await?;
        mailboxes
            .into_iter()
            .find(|m| m.name == name)
            .ok_or_else(|| mailbox_not_found(name))
    }

    async fn get_mailbox_status(&self, name: &str) -> Result<MailboxStatus, Error> {
        let raw = self.resolve_raw(name).await?;
        self.conn
            .run(|s| {
                let raw = raw.clone();
                Box::pin(async move {
                    let mbox =
                        s.status(&raw, "(MESSAGES UNSEEN RECENT)")
                            .await
                            .map_err(|e| match e {
                                async_imap::error::Error::No(m) => mailbox_not_found(&m),
                                other => Error::Imap(other.to_string()),
                            })?;
                    Ok(MailboxStatus {
                        total: mbox.exists,
                        unread: mbox.unseen.unwrap_or(0),
                        recent: mbox.recent,
                    })
                })
            })
            .await
    }

    async fn search_messages(&self, request: &SearchRequest) -> Result<Vec<MessageSummary>, Error> {
        let query = translate::to_imap_query(request);
        let mailboxes = self.search_mailboxes(request).await?;
        let mut results = Vec::new();
        for raw in mailboxes {
            let mut found = self.search_in_mailbox(&raw, &query, 0, None).await?;
            results.append(&mut found);
        }
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit;
        Ok(apply_paging(results, offset, limit))
    }

    async fn get_message(&self, id: &str) -> Result<Message, Error> {
        self.fetch_message_by_id(id).await
    }

    async fn get_headers(&self, id: &str) -> Result<Vec<gmail_mcp_core::model::Header>, Error> {
        let id = id.to_string();
        let app = AppId::from(id.as_str());
        let provider_id = app
            .provider_id()
            .ok_or_else(|| invalid_request(format!("malformed message id `{id}`")))?;
        let raw = self.resolve_raw("all").await?;
        self.conn
            .run(|s| {
                let id = id.clone();
                let raw = raw.clone();
                let provider_id = provider_id.clone();
                Box::pin(async move {
                    s.examine(&raw).await.map_err(|e| match e {
                        async_imap::error::Error::No(m) => mailbox_not_found(&m),
                        other => Error::Imap(other.to_string()),
                    })?;
                    let uids = s
                        .uid_search(format!("X-GM-MSGID {provider_id}"))
                        .await
                        .map_err(imap_err)?;
                    let uid = uids
                        .into_iter()
                        .next()
                        .ok_or_else(|| message_not_found(&id))?;
                    let fetched =
                        fetch::uid_fetch(s, &uid.to_string(), "(BODY.PEEK[HEADER])").await?;
                    let f = fetched
                        .into_iter()
                        .next()
                        .ok_or_else(|| message_not_found(&id))?;
                    let header = f
                        .header
                        .ok_or_else(|| Error::Mime("fetch returned no header".into()))?;
                    mime::parse_headers(&header)
                })
            })
            .await
    }

    async fn get_thread(&self, id: &str, full: bool) -> Result<Thread, Error> {
        let id = id.to_string();
        let app = AppId::from(id.as_str());
        let thrid = app
            .provider_id()
            .ok_or_else(|| invalid_request(format!("malformed thread id `{id}`")))?;
        let raw = self.resolve_raw("all").await?;
        let alias = self.alias.clone();
        let renderer = self.renderer.clone();
        let query = if full {
            "(BODY.PEEK[] FLAGS X-GM-MSGID X-GM-THRID X-GM-LABELS)"
        } else {
            "(BODY.PEEK[HEADER] BODY.PEEK[TEXT]<0.300> FLAGS X-GM-MSGID X-GM-THRID)"
        };
        let fetched = self
            .conn
            .run(|s| {
                let id = id.clone();
                let raw = raw.clone();
                let thrid = thrid.clone();
                let query = query.to_string();
                Box::pin(async move {
                    s.examine(&raw).await.map_err(|e| match e {
                        async_imap::error::Error::No(m) => mailbox_not_found(&m),
                        other => Error::Imap(other.to_string()),
                    })?;
                    let uids = s
                        .uid_search(format!("X-GM-THRID {thrid}"))
                        .await
                        .map_err(imap_err)?;
                    if uids.is_empty() {
                        return Err(thread_not_found(&id));
                    }
                    let mut uids: Vec<u32> = uids.into_iter().collect();
                    uids.sort_unstable();
                    fetch::uid_fetch(s, &join_uids(&uids), &query).await
                })
            })
            .await?;

        let mut entries: Vec<ThreadEntry> = Vec::new();
        for f in fetched {
            if full {
                let body = f
                    .body
                    .ok_or_else(|| Error::Mime("fetch returned no body".into()))?;
                let ctx = MessageContext {
                    account: alias.clone(),
                    mailbox: normalize_name(&raw),
                    provider: ProviderIds {
                        gmail_msgid: f.gmail_msgid,
                        gmail_thrid: f.gmail_thrid,
                        uid: f.uid,
                        seq: Some(f.seq),
                        message_id: None,
                    },
                    flags: f.flags.into_iter().map(Flag).collect(),
                    labels: f.labels,
                    size: f.size.map(|s| s as u64).unwrap_or(body.len() as u64),
                };
                let msg = mime::parse_message(&body, ctx, &*renderer)?;
                entries.push(ThreadEntry {
                    id: msg.id.clone(),
                    subject: msg.subject.clone(),
                    sender: msg.sender.clone(),
                    date: msg.date,
                    summary: msg.plain_text.as_deref().map(first_line),
                    message: Some(msg),
                });
            } else {
                let summary = summary_from_fetch(&alias, f, &raw);
                entries.push(ThreadEntry {
                    id: summary.id.clone(),
                    subject: summary.subject.clone(),
                    sender: summary.sender.clone(),
                    date: summary.date,
                    summary: summary.snippet.clone(),
                    message: None,
                });
            }
        }

        entries.sort_by_key(|e| e.date);
        let participants = unique_senders(&entries);
        let date_range = date_range(&entries);
        let subject = entries
            .iter()
            .find_map(|e| e.subject.clone())
            .or_else(|| Some("(no subject)".to_string()));

        Ok(Thread {
            id: AppId::new(Kind::Thread, &thrid),
            subject,
            participants,
            message_count: entries.len(),
            date_range,
            correlation: "gmail-thrid".to_string(),
            messages: entries,
        })
    }

    async fn list_attachments(&self, id: &str) -> Result<Vec<AttachmentMeta>, Error> {
        let message = self.fetch_message_by_id(id).await?;
        Ok(message.attachments)
    }

    async fn get_attachment(
        &self,
        id: &str,
    ) -> Result<gmail_mcp_core::model::AttachmentResult, Error> {
        let app = AppId::from(id);
        let provider = app
            .provider_id()
            .ok_or_else(|| invalid_request(format!("malformed attachment id `{id}`")))?;
        let (msgid, part_index) = provider
            .split_once('.')
            .ok_or_else(|| invalid_request(format!("malformed attachment id `{id}`")))?;
        let message = self
            .fetch_message_by_id(&AppId::new(Kind::Message, msgid).to_string())
            .await?;
        let raw = message
            .raw_mime
            .as_deref()
            .ok_or_else(|| Error::Mime("message has no raw MIME".into()))?;
        let parsed = mailparse::parse_mail(raw)
            .map_err(|e| Error::Mime(format!("failed to parse message: {e}")))?;
        let part = mime::find_part(&parsed, part_index).ok_or_else(|| attachment_not_found(id))?;
        let data = part
            .get_body_raw()
            .map_err(|e| Error::Mime(format!("failed to decode attachment: {e}")))?;
        let meta = message
            .attachments
            .iter()
            .find(|a| a.part_index == part_index)
            .cloned()
            .ok_or_else(|| attachment_not_found(id))?;

        if self.policy.should_return_inline(data.len() as u64) {
            Ok(gmail_mcp_core::model::AttachmentResult::Inline { data, meta })
        } else {
            let path = self.policy.write_temp(&meta.filename, &data)?;
            Ok(gmail_mcp_core::model::AttachmentResult::TempFile { path, meta })
        }
    }
}

fn summary_from_fetch(alias: &str, f: FetchedMessage, raw_mailbox: &str) -> MessageSummary {
    let headers = f
        .header
        .as_deref()
        .and_then(|h| mime::parse_headers(h).ok())
        .unwrap_or_default();
    let subject = header(&headers, "Subject");
    let sender = header(&headers, "From")
        .and_then(|v| mailparse::addrparse(&v).ok())
        .and_then(|list| {
            list.iter().find_map(|a| match a {
                mailparse::MailAddr::Single(info) => Some(Address {
                    name: info.display_name.clone(),
                    email: info.addr.clone(),
                }),
                _ => None,
            })
        });
    let date = header(&headers, "Date")
        .and_then(|d| chrono::DateTime::parse_from_rfc2822(&d).ok())
        .map(|d| d.with_timezone(&chrono::Utc));
    let snippet = f
        .text_preview
        .as_deref()
        .map(|t| String::from_utf8_lossy(t).trim().to_string())
        .filter(|s| !s.is_empty());

    let msgid = f.gmail_msgid.map(|m| m.to_string());
    let uid = f.uid.map(|u| u.to_string());
    let provider_id = msgid.clone().or(uid).unwrap_or_default();
    let thrid = f.gmail_thrid.map(|t| t.to_string());

    MessageSummary {
        id: AppId::new(Kind::Message, &provider_id),
        thread_id: AppId::new(
            Kind::Thread,
            &thrid.clone().unwrap_or_else(|| provider_id.clone()),
        ),
        account: alias.to_string(),
        mailbox: normalize_name(raw_mailbox),
        subject,
        sender,
        date,
        flags: f.flags.into_iter().map(Flag).collect(),
        snippet,
    }
}

fn header(headers: &[gmail_mcp_core::model::Header], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.clone())
        .filter(|v| !v.is_empty())
}

fn join_uids(uids: &[u32]) -> String {
    uids.iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn apply_paging<T>(mut items: Vec<T>, offset: usize, limit: Option<usize>) -> Vec<T> {
    if offset > 0 {
        items = items.into_iter().skip(offset).collect();
    }
    if let Some(limit) = limit {
        items.truncate(limit);
    }
    items
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").trim().to_string()
}

fn unique_senders(entries: &[ThreadEntry]) -> Vec<Address> {
    let mut seen = std::collections::HashSet::new();
    entries
        .iter()
        .filter_map(|e| e.sender.clone())
        .filter(|a| seen.insert(a.email.clone()))
        .collect()
}

fn date_range(
    entries: &[ThreadEntry],
) -> Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
    let dates: Vec<_> = entries.iter().filter_map(|e| e.date).collect();
    let min = dates.iter().min()?;
    let max = dates.iter().max()?;
    Some((*min, *max))
}
