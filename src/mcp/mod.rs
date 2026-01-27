//! MCP (Model Context Protocol) server implementation for browser automation
//!
//! This module provides rmcp-compatible tools by wrapping the existing tool implementations.

pub mod handler;
pub use handler::BrowserServer;

use crate::tools::{self, Tool, ToolContext, ToolResult as InternalToolResult};
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    tool, tool_router,
};

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

/// Macro to register MCP tools by automatically generating wrapper functions
macro_rules! register_mcp_tools {
    ($($mcp_name:ident => $tool_type:ty, $description:expr);* $(;)?) => {
        #[tool_router]
        impl BrowserServer {
            $(
                #[tool(description = $description)]
                async fn $mcp_name(
                    &self,
                    params: Parameters<<$tool_type as Tool>::Params>,
                ) -> Result<CallToolResult, McpError> {
                    let mut context = ToolContext::new();
                    let tool = <$tool_type>::default();
                    let result = tool.execute_typed(params.0, &mut context)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    convert_result(result)
                }
            )*
        }
    };
}

// Register all MCP tools using the macro
register_mcp_tools! {
    web_search => tools::web_search::WebSearchTool, "Perform a web search using Azure AI Search. Returns search results with response text and references.";
}
