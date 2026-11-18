use crate::config::{ToolSection, active_linkup_config};
use crate::error::{BrowserError, Result};
use crate::providers;
use crate::providers::linkup_search::LinkupSearchParams;
use crate::tools::{Tool, ToolContext, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for web search
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchParams {
    /// The search query text
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
struct LinkupWebSearchParams {
    #[serde(flatten)]
    search: WebSearchParams,
    #[serde(flatten)]
    linkup: LinkupSearchParams,
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

    fn parameters_schema(&self) -> Value {
        let Some(config) = active_linkup_config() else {
            return schemars::schema_for!(WebSearchParams).to_value();
        };

        let mut schema = schemars::schema_for!(LinkupWebSearchParams).to_value();
        if let Some(properties) = schema["properties"].as_object_mut() {
            for name in config.disabled_parameters() {
                properties.remove(name);
            }
        }
        schema
    }

    async fn execute(&self, params: Value, context: &mut ToolContext<'_>) -> Result<ToolResult> {
        let invalid_params =
            |error| BrowserError::InvalidArgument(format!("Invalid parameters: {error}"));
        let Some(config) = active_linkup_config() else {
            let params = serde_json::from_value(params).map_err(invalid_params)?;
            return self.execute_typed(params, context).await;
        };

        if let Some(name) = config
            .disabled_parameters()
            .find(|name| params.get(name).is_some())
        {
            return Err(BrowserError::InvalidArgument(format!(
                "LinkUp parameter '{name}' is disabled"
            )));
        }

        let params: LinkupWebSearchParams =
            serde_json::from_value(params).map_err(invalid_params)?;
        providers::linkup_search::execute_with_params(config, params.search.query, params.linkup)
            .await
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
            SearchProvider::Linkup => providers::linkup_search::execute(params.query).await,
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
    Linkup,
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
        "linkup_search" => Ok(SearchProvider::Linkup),
        other => Err(BrowserError::ConfigError(format!(
            "Unsupported provider '{}'",
            other
        ))),
    }
}
