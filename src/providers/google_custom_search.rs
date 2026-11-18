use crate::config::{ToolSection, get_global_config};
use crate::error::{BrowserError, Result};
use crate::tools::ToolResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const TOOL_NAME: &str = "google_custom_search";
const SEARCH_URL: &str = "https://customsearch.googleapis.com/customsearch/v1";

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<SearchResult>,
}

#[derive(Deserialize, Serialize)]
struct SearchResult {
    title: String,
    #[serde(rename(deserialize = "link"))]
    url: String,
    #[serde(default)]
    snippet: String,
}

/// Execute a Google Custom Search request.
pub async fn execute(query: String) -> Result<ToolResult> {
    let config = get_global_config().await;
    let ToolSection::Search(search_config) = &config.tool;
    let google = search_config.google_custom_search.as_ref().ok_or_else(|| {
        BrowserError::ConfigError(
            "Google Custom Search provider is not configured in [tool.google_custom_search]"
                .to_string(),
        )
    })?;

    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(request_error)?
        .get(SEARCH_URL)
        .query(&[
            ("key", google.api_key.as_str()),
            ("cx", google.search_engine_id.as_str()),
            ("q", query.as_str()),
        ])
        .send()
        .await
        .map_err(request_error)?;

    if !response.status().is_success() {
        return Err(BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Google Custom Search returned HTTP {}", response.status()),
        });
    }

    let search: SearchResponse = response.json().await.map_err(request_error)?;
    Ok(ToolResult::success_with(
        serde_json::json!({ "results": search.items }),
    ))
}

fn request_error(error: reqwest::Error) -> BrowserError {
    BrowserError::ToolExecutionFailed {
        tool: TOOL_NAME.to_string(),
        reason: error.without_url().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::SearchResponse;
    use serde_json::json;

    #[test]
    fn converts_google_items_to_search_results() {
        let response: SearchResponse = serde_json::from_value(json!({
            "items": [{
                "title": "Rust",
                "link": "https://www.rust-lang.org/",
                "snippet": "The Rust programming language.",
                "htmlTitle": "<b>Rust</b>"
            }]
        }))
        .unwrap();

        assert_eq!(
            serde_json::to_value(response.items).unwrap(),
            json!([{
                "title": "Rust",
                "url": "https://www.rust-lang.org/",
                "snippet": "The Rust programming language."
            }])
        );
    }

    #[test]
    fn handles_google_response_without_items() {
        let response: SearchResponse =
            serde_json::from_value(json!({ "searchInformation": { "totalResults": "0" } }))
                .unwrap();

        assert!(response.items.is_empty());
    }
}
