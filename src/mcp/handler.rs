//! ServerHandler implementation for WebSearchServer

use log::debug;
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo},
    tool_handler,
};

/// MCP Server wrapper for web search
///
/// This struct provides web search capabilities via Azure AI Search
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
        ServerInfo {
            instructions: Some("Web Search MCP Server - Provides web search capabilities via Azure AI Search".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
