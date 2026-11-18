//! ServerHandler implementation for WebSearchServer

use crate::tools::{Tool, WebSearchTool};
use log::debug;
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo},
    tool_handler,
};
use serde_json::Value;
use std::sync::Arc;

/// MCP Server wrapper for web search
///
/// This struct provides web search capabilities via configurable providers
#[derive(Clone)]
pub struct BrowserServer {
    tool_router: ToolRouter<Self>,
}

impl BrowserServer {
    /// Create a new web search server
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            tool_router: Self::tool_router(),
        })
    }

    fn tool_router() -> ToolRouter<Self> {
        let mut tool_router = Self::base_tool_routes();
        let config = crate::config::get_global_config_sync();
        let configured_tool_name = match &config.tool {
            crate::config::ToolSection::Search(tool_config) => tool_config.tool_name().to_string(),
        };

        if let Some(mut route) = tool_router.map.remove("web_search") {
            let Value::Object(schema) = WebSearchTool.parameters_schema() else {
                unreachable!("WebSearchTool parameters must have an object schema");
            };
            route.attr.input_schema = Arc::new(schema);
            route.attr.name = configured_tool_name.clone().into();
            tool_router.map.insert(configured_tool_name.into(), route);
        }

        tool_router
    }
}

impl Default for BrowserServer {
    fn default() -> Self {
        Self::new().expect("Failed to create default web search server")
    }
}

impl Drop for BrowserServer {
    fn drop(&mut self) {
        debug!("WebSearchServer dropped");
    }
}

#[tool_handler]
impl ServerHandler for BrowserServer {
    fn get_info(&self) -> ServerInfo {
        let _ = &self.tool_router;
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Web Search MCP Server - Provides web search capabilities via Azure AI Search, Perplexity, GenAI Responses, Google Custom Search, and LinkUp."
                    .to_string(),
            )
    }
}
