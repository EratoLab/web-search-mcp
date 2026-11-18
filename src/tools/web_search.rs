use crate::config::ToolSection;
use crate::error::{BrowserError, Result};
use crate::providers;
use crate::tools::{Tool, ToolContext, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for web search
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchParams {
    /// The search query text
    pub query: String,
}

/// Generic web search tool with provider selection configured at startup.
#[derive(Default)]
pub struct WebSearchTool;

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    type Params = WebSearchParams;

    fn name(&self) -> &str {
        "web_search"
    }

    async fn execute_typed(
        &self,
        params: WebSearchParams,
        _context: &mut ToolContext<'_>,
    ) -> Result<ToolResult> {
        match resolve_active_provider().await? {
            SearchProvider::Azure => providers::azure_ai_search::execute(params.query).await,
            SearchProvider::Perplexity => providers::perplexity_search::execute(params.query).await,
            SearchProvider::GoogleCustomSearch => {
                providers::google_custom_search::execute(params.query).await
            }
            SearchProvider::GenAIResponses => {
                providers::genai_responses::execute(params.query).await
            }
        }
    }
}

enum SearchProvider {
    Azure,
    Perplexity,
    GenAIResponses,
    GoogleCustomSearch,
}

async fn resolve_active_provider() -> Result<SearchProvider> {
    let config = crate::config::get_global_config().await;
    let tool_config = match &config.tool {
        ToolSection::Search(tool_config) => tool_config,
    };

    match tool_config.resolved_provider()?.as_str() {
        "azure_ai_search_web" => Ok(SearchProvider::Azure),
        "perplexity_search" => Ok(SearchProvider::Perplexity),
        "genai_responses" => Ok(SearchProvider::GenAIResponses),
        "google_custom_search" => Ok(SearchProvider::GoogleCustomSearch),
        other => Err(BrowserError::ConfigError(format!(
            "Unsupported provider '{}'",
            other
        ))),
    }
}
