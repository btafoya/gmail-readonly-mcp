//! IMAP connection lifecycle: TLS connect, app-password authentication, and
//! reconnect-on-failure.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_imap::Session;
use gmail_mcp_core::config::AccountConfig;
use gmail_mcp_core::error::Error;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

/// The connection stream: TLS (the normal case) or plain TCP.
pub enum Conn {
    Tls(TlsStream<TcpStream>),
    Plain(TcpStream),
}

impl AsyncRead for Conn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Tls(s) => Pin::new(s).poll_read(cx, buf),
            Conn::Plain(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Conn {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Conn::Tls(s) => Pin::new(s).poll_write(cx, buf),
            Conn::Plain(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Tls(s) => Pin::new(s).poll_flush(cx),
            Conn::Plain(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Tls(s) => Pin::new(s).poll_shutdown(cx),
            Conn::Plain(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl Unpin for Conn {}

impl fmt::Debug for Conn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Conn::Tls(_) => write!(f, "Conn::Tls"),
            Conn::Plain(_) => write!(f, "Conn::Plain"),
        }
    }
}

/// The authenticated IMAP session type.
pub type ImapSession = Session<Conn>;

/// A lazily-established, reconnectable IMAP connection for one account.
pub struct Connection {
    account: AccountConfig,
    session: tokio::sync::Mutex<Option<ImapSession>>,
}

impl Connection {
    pub fn new(account: AccountConfig) -> Self {
        Connection {
            account,
            session: tokio::sync::Mutex::new(None),
        }
    }

    /// Run an operation against the session, reconnecting once on a retryable
    /// failure (stale connection, network error).
    pub async fn run<T, F>(&self, op: F) -> Result<T, Error>
    where
        F: for<'a> Fn(
            &'a mut ImapSession,
        )
            -> Pin<Box<dyn std::future::Future<Output = Result<T, Error>> + Send + 'a>>,
    {
        let mut guard = self.session.lock().await;
        if guard.is_none() {
            *guard = Some(self.connect().await?);
        }
        match op(guard.as_mut().unwrap()).await {
            Ok(value) => Ok(value),
            Err(e) if e.is_retryable() => {
                tracing::warn!(error = %e, "imap operation failed; reconnecting");
                *guard = Some(self.connect().await?);
                op(guard.as_mut().unwrap()).await
            }
            Err(e) => Err(e),
        }
    }

    /// Drop the current session (used on shutdown).
    pub fn drop_session(&self) {
        *self.session.blocking_lock() = None;
    }

    async fn connect(&self) -> Result<ImapSession, Error> {
        let timeout = self.account.connect_timeout();
        let host = self.account.imap_host.clone();
        let port = self.account.imap_port;

        let tcp = tokio::time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
            .await
            .map_err(|_| Error::Timeout(format!("connecting to {host}:{port}")))?
            .map_err(|e| Error::Network(format!("connect to {host}:{port}: {e}")))?;

        let stream: Conn = if self.account.tls {
            let connector = native_tls::TlsConnector::new()
                .map_err(|e| Error::Network(format!("TLS setup failed: {e}")))?;
            let connector = tokio_native_tls::TlsConnector::from(connector);
            let tls = tokio::time::timeout(timeout, connector.connect(&host, tcp))
                .await
                .map_err(|_| Error::Timeout(format!("TLS handshake with {host}")))?
                .map_err(|e| Error::Network(format!("TLS handshake with {host}: {e}")))?;
            Conn::Tls(tls)
        } else {
            Conn::Plain(tcp)
        };

        let client = async_imap::Client::new(stream);
        let session = client
            .login(&self.account.email, &self.account.app_password)
            .await
            .map_err(|(e, _)| {
                Error::Auth(format!("login as {} failed: {e}", self.account.email))
            })?;
        tracing::debug!(account = %self.account.email, "imap session established");
        Ok(session)
    }
}

/// A helper for timeouts around session operations.
pub async fn with_timeout<T>(
    timeout: Duration,
    fut: impl std::future::Future<Output = Result<T, Error>>,
) -> Result<T, Error> {
    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| Error::Timeout("operation timed out".into()))?
}
