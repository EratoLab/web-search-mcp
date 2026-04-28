use crate::config::ToolSection;
use crate::error::{BrowserError, Result};
use crate::tools::ToolResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const TOOL_NAME: &str = "perplexity_search";

#[derive(Debug, Clone)]
struct PerplexityConfigRuntime {
    api_key: String,
    base_url: String,
    country: Option<String>,
    max_results: Option<u32>,
    max_tokens: Option<u32>,
    max_tokens_per_page: Option<u32>,
    search_language_filter: Option<Vec<String>>,
    search_domain_filter: Option<Vec<String>>,
    last_updated_after_filter: Option<String>,
    last_updated_before_filter: Option<String>,
    search_after_date_filter: Option<String>,
    search_before_date_filter: Option<String>,
    search_recency_filter: Option<String>,
}

/// Execute a Perplexity search request.
pub async fn execute(query: String) -> Result<ToolResult> {
    let search_config = resolve_perplexity_config().await?;

    let url = format!("{}/search", search_config.base_url.trim_end_matches('/'));
    let mut request_body = serde_json::json!({
        "query": query
    });

    merge_optional_param(&mut request_body, "country", search_config.country.as_ref());
    merge_optional_param(
        &mut request_body,
        "max_results",
        search_config.max_results.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "max_tokens",
        search_config.max_tokens.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "max_tokens_per_page",
        search_config.max_tokens_per_page.as_ref(),
    );
    merge_optional_string_vec(
        &mut request_body,
        "search_language_filter",
        search_config.search_language_filter.as_ref(),
    );
    merge_optional_string_vec(
        &mut request_body,
        "search_domain_filter",
        search_config.search_domain_filter.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "last_updated_after_filter",
        search_config.last_updated_after_filter.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "last_updated_before_filter",
        search_config.last_updated_before_filter.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "search_after_date_filter",
        search_config.search_after_date_filter.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "search_before_date_filter",
        search_config.search_before_date_filter.as_ref(),
    );
    merge_optional_param(
        &mut request_body,
        "search_recency_filter",
        search_config.search_recency_filter.as_ref(),
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Failed to build Perplexity reqwest client: {e}"),
        })?;
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", search_config.api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Failed to send request to Perplexity: {}", e),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!(
                "Perplexity request failed with status {}: {}",
                status, error_text
            ),
        });
    }

    let search_response: PerplexitySearchResponse =
        response
            .json()
            .await
            .map_err(|e| BrowserError::ToolExecutionFailed {
                tool: TOOL_NAME.to_string(),
                reason: format!("Failed to parse Perplexity response: {}", e),
            })?;

    Ok(ToolResult::success_with(serde_json::json!({
        "results": search_response.results,
        "id": search_response.id,
        "server_time": search_response.server_time,
    })))
}

async fn resolve_perplexity_config() -> Result<PerplexityConfigRuntime> {
    let config = crate::config::get_global_config().await;
    let tool_config = match &config.tool {
        ToolSection::Search(tool_config) => tool_config,
    };

    let perplexity = tool_config.perplexity_search.as_ref().ok_or_else(|| {
        BrowserError::ConfigError(
            "Perplexity provider is not configured in [tool.perplexity_search]".to_string(),
        )
    })?;

    Ok(PerplexityConfigRuntime {
        api_key: perplexity.api_key.clone(),
        base_url: perplexity.base_url.clone(),
        country: perplexity.country.clone(),
        max_results: perplexity.max_results,
        max_tokens: perplexity.max_tokens,
        max_tokens_per_page: perplexity.max_tokens_per_page,
        search_language_filter: perplexity.search_language_filter.clone(),
        search_domain_filter: perplexity.search_domain_filter.clone(),
        last_updated_after_filter: perplexity.last_updated_after_filter.clone(),
        last_updated_before_filter: perplexity.last_updated_before_filter.clone(),
        search_after_date_filter: perplexity.search_after_date_filter.clone(),
        search_before_date_filter: perplexity.search_before_date_filter.clone(),
        search_recency_filter: perplexity.search_recency_filter.clone(),
    })
}

fn merge_optional_param<T: serde::Serialize>(body: &mut Value, key: &str, value: Option<&T>) {
    if let Some(value) = value {
        if let Some(map) = body.as_object_mut() {
            map.insert(
                key.to_string(),
                serde_json::to_value(value).unwrap_or(Value::Null),
            );
        }
    }
}

fn merge_optional_string_vec(body: &mut Value, key: &str, values: Option<&Vec<String>>) {
    if let Some(values) = values {
        if let Some(map) = body.as_object_mut() {
            map.insert(key.to_string(), serde_json::json!(values));
        }
    }
}

/// Perplexity Search API response structure
#[derive(Debug, Serialize, Deserialize)]
struct PerplexitySearchResponse {
    #[serde(default)]
    results: Vec<PerplexitySearchResult>,
    id: Option<String>,
    server_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerplexitySearchResult {
    title: String,
    url: String,
    snippet: String,
    date: Option<String>,
    last_updated: Option<String>,
}
