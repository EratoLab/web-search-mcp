//! # web-search-mcp
//!
//! A Model Context Protocol (MCP) server for web search using Azure AI Search.
//!
//! ## Features
//!
//! - **MCP Server**: Model Context Protocol server for web search
//! - **Azure AI Search Integration**: Uses Azure AI Search knowledge bases for web search
//! - **Simple Configuration**: Configure via environment variables
//!
//! ## MCP Server
//!
//! ### Running the MCP Server
//!
//! ```bash
//! # Set required environment variables
//! export AZURE_AI_SEARCH_BASE_URL="https://your-search.search.windows.net"
//! export AZURE_AI_SEARCH_KB_NAME="your-kb-name"
//! export AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME="web-search"
//! export AZURE_AI_SEARCH_API_KEY="your-api-key"
//!
//! # Run the server
//! cargo run --bin mcp-server
//! ```
//!
//! ## Module Overview
//!
//! - [`tools`]: Web search tools
//! - [`error`]: Error types and result aliases
//! - [`mcp`]: **Model Context Protocol server** (requires `mcp-handler` feature)

pub mod error;
pub mod tools;

#[cfg(feature = "mcp-handler")]
pub mod mcp;

pub use error::{BrowserError, Result};
pub use tools::{Tool, ToolContext, ToolRegistry, ToolResult};

#[cfg(feature = "mcp-handler")]
pub use mcp::BrowserServer;
#[cfg(feature = "mcp-handler")]
pub use rmcp::ServiceExt;
