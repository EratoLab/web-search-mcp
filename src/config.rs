//! Configuration module for Web Search MCP Server
//!
//! This module handles loading and managing configuration from TOML files.

use crate::error::{BrowserError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

/// Global configuration storage
static GLOBAL_CONFIG: OnceLock<Arc<ServerConfig>> = OnceLock::new();

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub tool: ToolSection,
    pub metadata: MetadataConfig,
    #[serde(default)]
    pub transport: TransportConfig,
}

/// Server metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    pub name: String,
    pub version: String,
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type: stdio, or http
    #[serde(default = "default_transport_type")]
    pub transport: String,

    /// Host address to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port for HTTP transport
    #[serde(default = "default_port")]
    pub port: u16,

    /// SSE endpoint path
    #[serde(default = "default_sse_path")]
    pub sse_path: String,

    /// SSE message post path
    #[serde(default = "default_sse_post_path")]
    pub sse_post_path: String,

    /// HTTP streamable endpoint path
    #[serde(default = "default_http_path")]
    pub http_path: String,
}

fn default_transport_type() -> String {
    "stdio".to_string()
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_sse_path() -> String {
    "/sse".to_string()
}

fn default_sse_post_path() -> String {
    "/message".to_string()
}

fn default_http_path() -> String {
    "/mcp".to_string()
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport: default_transport_type(),
            host: default_host(),
            port: default_port(),
            sse_path: default_sse_path(),
            sse_post_path: default_sse_post_path(),
            http_path: default_http_path(),
        }
    }
}

/// Tool configuration with provider-specific variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolSection {
    Search(SearchToolConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchToolConfig {
    /// Tool name exposed via MCP. Defaults to `web_search`.
    #[serde(default = "default_tool_name")]
    pub tool_name: String,

    /// Optional default provider if both are configured.
    #[serde(default)]
    pub default_provider: Option<String>,

    /// Azure AI Search provider configuration.
    #[serde(default)]
    pub azure_ai_search_web: Option<AzureAiSearchWebConfig>,

    /// Perplexity search provider configuration.
    #[serde(default)]
    pub perplexity_search: Option<PerplexitySearchConfig>,

    /// GenAI Responses (OpenAI Responses API) search provider configuration.
    #[serde(default)]
    pub genai_responses: Option<GenAIResponsesSearchConfig>,
}

/// Azure AI Search Web provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureAiSearchWebConfig {
    pub base_url: String,
    pub kb_name: String,
    pub knowledge_source_name: String,
    pub api_key: String,

    /// Optional knowledge source kind (web or searchIndex).
    pub knowledge_source_kind: Option<String>,
}

/// Perplexity search provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerplexitySearchConfig {
    pub api_key: String,

    /// Base URL for Perplexity API.
    #[serde(default = "default_perplexity_base_url")]
    pub base_url: String,

    pub country: Option<String>,
    pub max_results: Option<u32>,
    pub max_tokens: Option<u32>,
    pub max_tokens_per_page: Option<u32>,
    pub search_language_filter: Option<Vec<String>>,
    pub search_domain_filter: Option<Vec<String>>,
    pub last_updated_after_filter: Option<String>,
    pub last_updated_before_filter: Option<String>,
    pub search_after_date_filter: Option<String>,
    pub search_before_date_filter: Option<String>,
    pub search_recency_filter: Option<String>,
}

/// GenAI Responses search provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenAIResponsesSearchConfig {
    /// Base URL for the Responses API endpoint.
    pub base_url: String,

    /// Model used for the Responses call.
    #[serde(alias = "model_name")]
    pub model: String,

    /// API key for the provider.
    pub api_key: String,

    /// Optional API version for providers that require it.
    pub api_version: Option<String>,

    /// Optional auth header name for the API key (defaults to `Authorization`).
    #[serde(default)]
    pub api_key_header: Option<String>,

    /// Optional system prompt used for responses requests.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Optional `max_uses` setting passed to the built-in web_search tool.
    #[serde(default)]
    pub tool_max_uses: Option<u32>,

    /// Optional allowlist passed to the built-in web_search tool.
    #[serde(default)]
    pub tool_allowed_domains: Option<Vec<String>>,

    /// Optional reasoning effort passed to the Responses API.
    /// If unset, defaults to "none".
    #[serde(default)]
    pub reasoning: Option<String>,

    /// Optional blocklist passed to the built-in web_search tool.
    #[serde(default)]
    pub tool_blocked_domains: Option<Vec<String>>,
}

fn default_perplexity_base_url() -> String {
    "https://api.perplexity.ai".to_string()
}

impl SearchToolConfig {
    pub fn configured_providers(&self) -> Vec<&'static str> {
        let mut providers = Vec::new();
        if self.azure_ai_search_web.is_some() {
            providers.push("azure_ai_search_web");
        }
        if self.perplexity_search.is_some() {
            providers.push("perplexity_search");
        }
        if self.genai_responses.is_some() {
            providers.push("genai_responses");
        }
        providers
    }

    pub fn configured_provider_count(&self) -> usize {
        self.configured_providers().len()
    }

    pub fn has_provider(&self, provider: &str) -> bool {
        match provider {
            "azure_ai_search_web" => self.azure_ai_search_web.is_some(),
            "perplexity_search" => self.perplexity_search.is_some(),
            "genai_responses" => self.genai_responses.is_some(),
            _ => false,
        }
    }

    pub fn tool_name(&self) -> &str {
        self.tool_name.as_str()
    }

    pub fn resolved_provider(&self) -> Result<String> {
        if let Some(default_provider) = &self.default_provider {
            if self.has_provider(default_provider) {
                return Ok(default_provider.to_string());
            }
            return Err(BrowserError::ConfigError(format!(
                "Invalid default_provider '{}'.",
                default_provider
            )));
        }

        let providers = self.configured_providers();
        match providers.as_slice() {
            [provider] => Ok((*provider).to_string()),
            _ => Err(BrowserError::ConfigError(
                "No provider selected. Configure `default_provider` when multiple providers are enabled."
                    .to_string(),
            )),
        }
    }
}

fn default_tool_name() -> String {
    "web_search".to_string()
}

/// Metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub prefix: String,
}

impl ServerConfig {
    /// Load configuration from file path with priority fallback
    ///
    /// Priority order:
    /// 1. Explicit path provided
    /// 2. MCP_SERVER_CONFIG environment variable
    /// 3. ./mcp-server.toml (default)
    #[cfg(feature = "mcp-server")]
    pub async fn load(explicit_path: Option<String>) -> Result<Self> {
        let config_path = Self::resolve_config_path(explicit_path)?;
        Self::load_from_file(&config_path).await
    }

    /// Load configuration from file path with priority fallback (blocking version)
    ///
    /// Priority order:
    /// 1. Explicit path provided
    /// 2. MCP_SERVER_CONFIG environment variable
    /// 3. ./mcp-server.toml (default)
    #[cfg(not(feature = "mcp-server"))]
    pub async fn load(explicit_path: Option<String>) -> Result<Self> {
        let config_path = Self::resolve_config_path(explicit_path)?;
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            BrowserError::ConfigError(format!(
                "Failed to read config file {}: {}",
                config_path.display(),
                e
            ))
        })?;

        let config: ServerConfig = toml::from_str(&content).map_err(|e| {
            BrowserError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                config_path.display(),
                e
            ))
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Resolve the configuration file path based on priority
    fn resolve_config_path(explicit_path: Option<String>) -> Result<PathBuf> {
        // 1. Check explicit path (highest priority)
        if let Some(path) = explicit_path {
            return Ok(PathBuf::from(path));
        }

        // 2. Check environment variable
        if let Ok(path) = std::env::var("MCP_SERVER_CONFIG") {
            return Ok(PathBuf::from(path));
        }

        // 3. Use default path
        let default_path = PathBuf::from("./mcp-server.toml");

        // Check if default file exists
        if !default_path.exists() {
            return Err(BrowserError::ConfigError(format!(
                "Configuration file not found. Searched in order:\n\
                 1. Explicit path via --config argument\n\
                 2. MCP_SERVER_CONFIG environment variable\n\
                 3. ./mcp-server.toml (default)\n\n\
                 Please create a configuration file. Example:\n\
                 [server]\n\
                 name = \"web-search-mcp\"\n\
                 version = \"0.1.0\"\n\n\
                 [tool]\n\
                 type = \"search\"\n\
                 tool_name = \"web_search\"\n\
                 default_provider = \"azure_ai_search_web\"\n\n\
                 [tool.azure_ai_search_web]\n\
                 base_url = \"https://your-search.search.windows.net\"\n\
                 kb_name = \"your-kb-name\"\n\
                 knowledge_source_name = \"web-search\"\n\
                 api_key = \"your-api-key\"\n\
                 knowledge_source_kind = \"web\"\n\n\
\n\
[tool.genai_responses]\n\
base_url = \"https://{{your-resource}}.services.ai.azure.com/openai/deployments/{{your-deployment}}\"\n\
model = \"gpt-4o\"\n\
api_key = \"your-genai-api-key\"\n\
api_version = \"2025-04-01-preview\"\n\
api_key_header = \"api-key\"\n\
reasoning = \"none\"\n\
system_prompt = \"Always use the web_search tool and return concise results.\"\n\
tool_max_uses = 3\n\
tool_allowed_domains = [\"rust-lang.org\", \"docs.rs\", \"github.com\"]\n\n\
                 [metadata]\n\
                 prefix = \"chat.erato/\"\n\
\n\
                 [tool.perplexity_search]\n\
                 api_key = \"your-perplexity-api-key\"\n\
                 base_url = \"https://api.perplexity.ai\"\n"
            )));
        }

        Ok(default_path)
    }

    /// Load configuration from a specific file
    #[cfg(feature = "mcp-server")]
    async fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BrowserError::ConfigError(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: ServerConfig = toml::from_str(&content).map_err(|e| {
            BrowserError::ConfigError(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            ))
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        let search_config = match &self.tool {
            ToolSection::Search(search_config) => search_config,
        };

        if !self.metadata.prefix.ends_with('/') {
            return Err(BrowserError::ConfigError(
                "Metadata prefix must end with '/'".to_string(),
            ));
        }

        // Validate transport type
        let valid_transports = ["stdio", "http"];
        if !valid_transports.contains(&self.transport.transport.as_str()) {
            return Err(BrowserError::ConfigError(format!(
                "Invalid transport type: '{}'. Must be one of: {}",
                self.transport.transport,
                valid_transports.join(", ")
            )));
        }

        // Validate provider configuration
        let configured: Vec<&'static str> = search_config.configured_providers();

        if configured.is_empty() {
            return Err(BrowserError::ConfigError(
                "No search providers configured. Configure at least one of [tool.azure_ai_search_web], [tool.perplexity_search], or [tool.genai_responses]."
                    .to_string(),
            ));
        }

        if configured.len() > 1 && search_config.default_provider.is_none() {
            return Err(BrowserError::ConfigError(
                "Multiple providers configured. Set `default_provider` to one of: azure_ai_search_web, perplexity_search, or genai_responses."
                    .to_string(),
            ));
        }

        if let Some(default_provider) = &search_config.default_provider {
            if !search_config.has_provider(default_provider) {
                return Err(BrowserError::ConfigError(format!(
                    "Invalid default_provider '{}'. Must be one of: {}",
                    default_provider,
                    configured.join(", ")
                )));
            }
        }

        Ok(())
    }
}

/// Set the global configuration
pub async fn set_global_config(config: Arc<ServerConfig>) {
    let _ = GLOBAL_CONFIG.get_or_init(|| config).clone();
}

/// Get the global configuration
pub async fn get_global_config() -> Arc<ServerConfig> {
    GLOBAL_CONFIG
        .get()
        .expect("Global config not initialized. Call set_global_config() first.")
        .clone()
}
