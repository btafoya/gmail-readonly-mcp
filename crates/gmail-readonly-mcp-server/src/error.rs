//! Mapping of core domain errors to MCP errors.
//!
//! `ServerError` is a local newtype so the orphan rule allows converting it
//! into `rmcp::ErrorData`.

use gmail_mcp_core::error::Error;
use rmcp::ErrorData;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::CallToolResponse;

/// A server-side error wrapping a core domain error.
#[derive(Debug)]
pub struct ServerError(pub Error);

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ServerError {}

impl From<Error> for ServerError {
    fn from(e: Error) -> Self {
        ServerError(e)
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(e: serde_json::Error) -> Self {
        ServerError(Error::Internal(format!("serialization failed: {e}")))
    }
}

impl IntoCallToolResult for ServerError {
    fn into_call_tool_result(self) -> Result<CallToolResponse, ErrorData> {
        Err(ErrorData::from(self))
    }
}

impl From<ServerError> for ErrorData {
    fn from(e: ServerError) -> Self {
        match e.0 {
            Error::InvalidRequest(msg) => ErrorData::invalid_params(msg, None),
            Error::AccountNotFound(alias, available) => ErrorData::invalid_params(
                format!("account not found: {alias} (available: {available})"),
                None,
            ),
            Error::MailboxNotFound(name) => {
                ErrorData::resource_not_found(format!("mailbox not found: {name}"), None)
            }
            Error::MessageNotFound(id) => {
                ErrorData::resource_not_found(format!("message not found: {id}"), None)
            }
            Error::ThreadNotFound(id) => {
                ErrorData::resource_not_found(format!("thread not found: {id}"), None)
            }
            Error::AttachmentNotFound(id) => {
                ErrorData::resource_not_found(format!("attachment not found: {id}"), None)
            }
            other => ErrorData::internal_error(other.to_string(), None),
        }
    }
}
