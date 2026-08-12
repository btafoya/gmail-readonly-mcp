//! Opaque, deterministic, reversible application IDs.
//!
//! Application IDs are derived from stable provider identifiers (Gmail
//! `X-GM-MSGID` / `X-GM-THRID`, or IMAP UIDs as fallback). They are opaque to
//! casual reading but reversible so the service layer can map an application
//! ID back to the provider identifier it was built from without keeping state.
//!
//! Format: `<kind>:<hex(provider_id)>` where `kind` is one of `m` (message),
//! `t` (thread), `a` (attachment). Hex is used instead of base64 so the
//! encoding is std-only and stable.

use serde::{Deserialize, Serialize};

/// The kind of entity an application ID refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Message,
    Thread,
    Attachment,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Kind::Message => "m",
            Kind::Thread => "t",
            Kind::Attachment => "a",
        }
    }

    fn from_prefix(s: &str) -> Option<Kind> {
        match s {
            "m" => Some(Kind::Message),
            "t" => Some(Kind::Thread),
            "a" => Some(Kind::Attachment),
            _ => None,
        }
    }
}

/// An opaque application identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId(String);

impl AppId {
    /// Build an application ID from a provider identifier.
    pub fn new(kind: Kind, provider_id: &str) -> Self {
        AppId(format!(
            "{}:{}",
            kind.prefix(),
            hex_encode(provider_id.as_bytes())
        ))
    }

    /// The kind of entity this ID refers to.
    pub fn kind(&self) -> Option<Kind> {
        let (prefix, _) = self.0.split_once(':')?;
        Kind::from_prefix(prefix)
    }

    /// The provider identifier this ID was built from, if it decodes.
    pub fn provider_id(&self) -> Option<String> {
        let (_, hex) = self.0.split_once(':')?;
        hex_decode(hex).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    }

    /// The raw string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for AppId {
    fn from(s: String) -> Self {
        AppId(s)
    }
}

impl From<&str> for AppId {
    fn from(s: &str) -> Self {
        AppId(s.to_string())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for (kind, id) in [
            (Kind::Message, "1278455344230334865"),
            (Kind::Thread, "1278455344230334865"),
            (Kind::Attachment, "1278455344230334865.1.2"),
        ] {
            let app = AppId::new(kind, id);
            assert_eq!(app.kind(), Some(kind));
            assert_eq!(app.provider_id().as_deref(), Some(id));
        }
    }

    #[test]
    fn deterministic() {
        let a = AppId::new(Kind::Message, "42");
        let b = AppId::new(Kind::Message, "42");
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_garbage() {
        let app = AppId::from("m:zz");
        assert_eq!(app.provider_id(), None);
        assert_eq!(AppId::from("nope").kind(), None);
    }
}
