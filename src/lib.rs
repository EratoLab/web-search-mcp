//! # web-search-mcp
//!
//! A Model Context Protocol (MCP) server for web search using configurable providers.
//!
//! ## Features
//!
//! - **MCP Server**: Model Context Protocol server for web search
//! - **Configurable Providers**: Configure multiple providers through TOML
//! - **Simple Configuration**: Configure via TOML file
//!
//! ## MCP Server
//!
//! ### Running the MCP Server
//!
//! ```bash
//! # Use the default config file path: ./mcp-server.toml
//! 
//! # Run the server
//! cargo run --bin mcp-server
//! ```
//!
//! ## Module Overview
//!
//! - [`tools`]: Web search tools
//! - [`error`]: Error types and result aliases
//! - [`config`]: Configuration management
//! - [`mcp`]: **Model Context Protocol server** (requires `mcp-handler` feature)

pub mod config;
pub mod error;
pub mod providers;
pub mod tools;

#[cfg(feature = "mcp-handler")]
pub mod mcp;

pub use error::{BrowserError, Result};
pub use tools::{Tool, ToolContext, ToolRegistry, ToolResult};

#[cfg(feature = "mcp-handler")]
pub use mcp::BrowserServer;
#[cfg(feature = "mcp-handler")]
pub use rmcp::ServiceExt;
