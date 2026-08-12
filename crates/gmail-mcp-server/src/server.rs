//! The MCP server handler: tools, resources, and prompts over the core
//! read-only mail service. No Gmail logic lives here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gmail_mcp_core::attachment::AttachmentPolicy;
use gmail_mcp_core::config::ConfigFile;
use gmail_mcp_core::error::{Error, invalid_request};
use gmail_mcp_core::model::AttachmentResult;
use gmail_mcp_core::render::{DefaultRenderer, MessageRenderer, RenderFormat};
use gmail_mcp_core::search::{SearchFilters, SearchRequest};
use gmail_mcp_core::service::MailService;
use gmail_mcp_imap::ImapService;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    GetPromptResponse, GetPromptResult, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ReadResourceResponse, ReadResourceResult, Resource, ResourceContents,
    ResourceTemplate,
};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ServerError;
use crate::prompts;
use crate::resources;

/// The MCP server handler.
pub struct GmailMcpServer {
    config: Arc<ConfigFile>,
    policy: Arc<AttachmentPolicy>,
    renderer: Arc<DefaultRenderer>,
    services: Mutex<HashMap<String, Arc<ImapService>>>,
}

impl GmailMcpServer {
    pub fn new(config: ConfigFile) -> Self {
        let policy = Arc::new(config.attachment_policy());
        GmailMcpServer {
            config: Arc::new(config),
            policy,
            renderer: Arc::new(DefaultRenderer::new()),
            services: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve an account alias (or the default) to a lazily-created service.
    fn service(&self, alias: Option<&str>) -> Result<Arc<ImapService>, Error> {
        let (account, alias) = self.config.resolve_account(alias)?;
        let mut services = self.services.lock().unwrap();
        if let Some(svc) = services.get(&alias) {
            return Ok(svc.clone());
        }
        let svc = Arc::new(ImapService::new(
            alias.clone(),
            account.clone(),
            (*self.policy).clone(),
            self.renderer.clone(),
        ));
        services.insert(alias.clone(), svc.clone());
        Ok(svc)
    }
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct AccountParams {
    pub account: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct MailboxParams {
    pub account: Option<String>,
    pub name: String,
}

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct SearchParams {
    pub account: Option<String>,
    /// Gmail-native search query (e.g. `from:alice has:attachment`).
    pub query: Option<String>,
    pub mailbox: Option<String>,
    pub sender: Option<String>,
    pub recipients: Option<String>,
    pub subject: Option<String>,
    pub text: Option<String>,
    /// ISO 8601 date, inclusive lower bound.
    pub from: Option<String>,
    /// ISO 8601 date, inclusive upper bound.
    pub to: Option<String>,
    pub has_attachment: Option<bool>,
    pub flags: Vec<String>,
    pub include_spam: bool,
    pub include_trash: bool,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct MessageParams {
    pub account: Option<String>,
    pub id: String,
    /// Output format: json (default), markdown, html, text, raw.
    pub format: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ThreadParams {
    pub account: Option<String>,
    pub id: String,
    /// Return complete messages instead of a summary.
    pub full: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct AttachmentParams {
    pub account: Option<String>,
    pub id: String,
}

#[tool_router]
impl GmailMcpServer {
    #[tool(name = "list_mailboxes", description = "List mailboxes for an account")]
    async fn list_mailboxes(
        &self,
        Parameters(p): Parameters<AccountParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let mailboxes = service.list_mailboxes().await?;
        Ok(Json(serde_json::to_value(mailboxes)?))
    }

    #[tool(
        name = "get_mailbox",
        description = "Get details for a normalized mailbox name"
    )]
    async fn get_mailbox(
        &self,
        Parameters(p): Parameters<MailboxParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let mailbox = service.get_mailbox(&p.name).await?;
        Ok(Json(serde_json::to_value(mailbox)?))
    }

    #[tool(
        name = "get_mailbox_status",
        description = "Get mailbox status (total, unread, recent)"
    )]
    async fn get_mailbox_status(
        &self,
        Parameters(p): Parameters<MailboxParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let status = service.get_mailbox_status(&p.name).await?;
        Ok(Json(serde_json::to_value(status)?))
    }

    #[tool(
        name = "search_messages",
        description = "Search messages (metadata-first)"
    )]
    async fn search_messages(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let request = SearchRequest {
            query: p.query,
            filters: SearchFilters {
                mailbox: p.mailbox,
                sender: p.sender,
                recipients: p.recipients,
                subject: p.subject,
                text: p.text,
                date_from: parse_date(p.from.as_deref())?,
                date_to: parse_date(p.to.as_deref())?,
                has_attachment: p.has_attachment,
                flags: p.flags,
                include_spam: p.include_spam,
                include_trash: p.include_trash,
            },
            limit: p.limit,
            offset: p.offset,
        };
        let results = service.search_messages(&request).await?;
        Ok(Json(serde_json::to_value(results)?))
    }

    #[tool(
        name = "get_message",
        description = "Get a complete message by application ID"
    )]
    async fn get_message(
        &self,
        Parameters(p): Parameters<MessageParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let message = service.get_message(&p.id).await?;
        let format = p
            .format
            .as_deref()
            .map(RenderFormat::parse)
            .unwrap_or(Some(RenderFormat::Json))
            .ok_or_else(|| invalid_request("invalid format"))?;
        if format == RenderFormat::Json {
            Ok(Json(serde_json::to_value(message)?))
        } else {
            let content = self.renderer.render_message(&message, format)?;
            Ok(Json(serde_json::json!({
                "id": message.id,
                "format": format_name(format),
                "content": content,
            })))
        }
    }

    #[tool(
        name = "get_thread",
        description = "Get a conversation/thread by application thread ID"
    )]
    async fn get_thread(
        &self,
        Parameters(p): Parameters<ThreadParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let thread = service.get_thread(&p.id, p.full).await?;
        Ok(Json(serde_json::to_value(thread)?))
    }

    #[tool(
        name = "list_attachments",
        description = "List attachment metadata for a message"
    )]
    async fn list_attachments(
        &self,
        Parameters(p): Parameters<MessageParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let attachments = service.list_attachments(&p.id).await?;
        Ok(Json(serde_json::to_value(attachments)?))
    }

    #[tool(
        name = "get_attachment",
        description = "Retrieve an attachment by application ID"
    )]
    async fn get_attachment(
        &self,
        Parameters(p): Parameters<AttachmentParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let result = service.get_attachment(&p.id).await?;
        let (filename, content_type, size) = {
            let m = result.meta();
            (m.filename.clone(), m.content_type.clone(), m.size)
        };
        let value = match result {
            AttachmentResult::Inline { data, .. } => serde_json::json!({
                "filename": filename,
                "content_type": content_type,
                "size": size,
                "inline": true,
                "data_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data),
            }),
            AttachmentResult::TempFile { path, .. } => serde_json::json!({
                "filename": filename,
                "content_type": content_type,
                "size": size,
                "inline": false,
                "path": path.to_string_lossy(),
            }),
        };
        Ok(Json(value))
    }

    #[tool(
        name = "get_headers",
        description = "Get message headers by application ID"
    )]
    async fn get_headers(
        &self,
        Parameters(p): Parameters<MessageParams>,
    ) -> Result<Json<Value>, ServerError> {
        let service = self.service(p.account.as_deref())?;
        let headers = service.get_headers(&p.id).await?;
        Ok(Json(serde_json::to_value(headers)?))
    }
}

// ---------------------------------------------------------------------------
// Resources and prompts
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for GmailMcpServer {
    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut resources = Vec::new();
        for alias in self.config.aliases() {
            resources.push(
                Resource::new(
                    format!("gmail://{alias}/mailboxes"),
                    format!("{alias} mailboxes"),
                )
                .with_mime_type("application/json")
                .with_description("Mailbox list for the account"),
            );
        }
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        let mut templates = Vec::new();
        for alias in self.config.aliases() {
            templates.push(
                ResourceTemplate::new(
                    format!("gmail://{alias}/mailboxes/{{name}}"),
                    format!("{alias} mailbox"),
                )
                .with_mime_type("application/json"),
            );
            templates.push(
                ResourceTemplate::new(
                    format!("gmail://{alias}/messages/{{id}}"),
                    format!("{alias} message"),
                )
                .with_mime_type("application/json"),
            );
            templates.push(
                ResourceTemplate::new(
                    format!("gmail://{alias}/threads/{{id}}"),
                    format!("{alias} thread"),
                )
                .with_mime_type("application/json"),
            );
            templates.push(
                ResourceTemplate::new(
                    format!("gmail://{alias}/attachments/{{id}}"),
                    format!("{alias} attachment"),
                )
                .with_mime_type("application/json"),
            );
        }
        Ok(ListResourceTemplatesResult::with_all_items(templates))
    }

    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let (account, target) = resources::parse_uri(&request.uri).ok_or_else(|| {
            ErrorData::resource_not_found(format!("unknown resource: {}", request.uri), None)
        })?;
        let service = self.service(Some(&account)).map_err(ServerError::from)?;
        let value = resources::read_target(&*service, &target)
            .await
            .map_err(ServerError::from)?;
        let contents = vec![ResourceContents::text(value.to_string(), request.uri)];
        Ok(ReadResourceResult::new(contents).into())
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::with_all_items(prompts::list_prompts()))
    }

    async fn get_prompt(
        &self,
        request: rmcp::model::GetPromptRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResponse, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        prompts::get_prompt(&request.name, &args)
            .map(GetPromptResult::into)
            .ok_or_else(|| {
                ErrorData::invalid_request(format!("unknown prompt: {}", request.name), None)
            })
    }
}

fn parse_date(s: Option<&str>) -> Result<Option<chrono::DateTime<chrono::Utc>>, Error> {
    match s {
        None => Ok(None),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| Some(d.with_timezone(&chrono::Utc)))
            .map_err(|_| invalid_request(format!("invalid date `{s}`; use ISO 8601"))),
    }
}

fn format_name(format: RenderFormat) -> &'static str {
    match format {
        RenderFormat::Json => "json",
        RenderFormat::Markdown => "markdown",
        RenderFormat::Html => "html",
        RenderFormat::Text => "text",
        RenderFormat::Raw => "raw",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_all_tools_and_no_mutation_tools() {
        let tools = GmailMcpServer::tool_router().list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        for expected in [
            "list_mailboxes",
            "get_mailbox",
            "get_mailbox_status",
            "search_messages",
            "get_message",
            "get_thread",
            "list_attachments",
            "get_attachment",
            "get_headers",
        ] {
            assert!(names.contains(&expected), "missing tool {expected}");
        }
        assert_eq!(names.len(), 9);
        // Read-only constraint: no mutation tools.
        for name in names {
            for forbidden in [
                "send", "delete", "move", "copy", "flag", "label", "mark", "create", "update",
            ] {
                assert!(!name.contains(forbidden), "mutation tool present: {name}");
            }
        }
    }
}
