//! MCP (Model Context Protocol) server implementation for browser automation
//!
//! This module provides rmcp-compatible tools by wrapping the existing tool implementations.

pub mod handler;
pub use handler::BrowserServer;

use crate::error::BrowserError;
use crate::tools::{Tool, ToolContext, ToolResult as InternalToolResult, WebSearchTool};
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, JsonObject},
    tool, tool_router,
};
use serde_json::Value;

/// Convert internal ToolResult to MCP CallToolResult
fn convert_result(result: InternalToolResult) -> Result<CallToolResult, McpError> {
    if result.success {
        let text = if let Some(data) = result.data {
            serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string())
        } else {
            "Success".to_string()
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    } else {
        let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
        Err(McpError::internal_error(error_msg, None))
    }
}

#[tool_router(router = base_tool_routes)]
impl BrowserServer {
    #[tool(
        description = "Perform a web search using the configured provider. Returns search results and metadata."
    )]
    async fn web_search(
        &self,
        Parameters(params): Parameters<JsonObject>,
    ) -> Result<CallToolResult, McpError> {
        let result = WebSearchTool
            .execute(Value::Object(params), &mut ToolContext::new())
            .await
            .map_err(|error| match error {
                BrowserError::InvalidArgument(message) => McpError::invalid_params(message, None),
                error => McpError::internal_error(error.to_string(), None),
            })?;
        convert_result(result)
    }
}
