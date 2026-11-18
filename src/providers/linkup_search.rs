use crate::config::{LinkupSearchConfig, LinkupSearchDepth, ToolSection, get_global_config};
use crate::error::{BrowserError, Result};
use crate::tools::ToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::num::NonZeroU32;
use std::time::Duration;

const TOOL_NAME: &str = "linkup_search";

#[derive(Default, Deserialize, JsonSchema)]
pub(crate) struct LinkupSearchParams {
    /// LinkUp search depth.
    depth: Option<LinkupSearchDepth>,
    /// Include image results alongside text results.
    include_images: Option<bool>,
    /// Include content published on or after this date (YYYY-MM-DD).
    from_date: Option<String>,
    /// Include content published on or before this date (YYYY-MM-DD).
    to_date: Option<String>,
    /// Maximum number of results to return.
    max_results: Option<NonZeroU32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest<'a> {
    q: &'a str,
    depth: LinkupSearchDepth,
    output_type: &'static str,
    include_images: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_domains: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_domains: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_date: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to_date: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<NonZeroU32>,
}

#[derive(Deserialize, Serialize)]
struct SearchResponse {
    // Preserve the fields provided by LinkUp for both text and image results.
    results: Vec<Value>,
}

/// Execute a LinkUp search request.
pub async fn execute(query: String) -> Result<ToolResult> {
    let config = get_global_config().await;
    let ToolSection::Search(search_config) = &config.tool;
    let linkup = search_config.linkup_search.as_ref().ok_or_else(|| {
        BrowserError::ConfigError(
            "LinkUp provider is not configured in [tool.linkup_search]".to_string(),
        )
    })?;
    execute_with_params(linkup, query, LinkupSearchParams::default()).await
}

pub(crate) async fn execute_with_params(
    linkup: &LinkupSearchConfig,
    query: String,
    params: LinkupSearchParams,
) -> Result<ToolResult> {
    let url = format!("{}/search", linkup.base_url.trim_end_matches('/'));
    let request = SearchRequest {
        q: &query,
        depth: params.depth.unwrap_or(linkup.depth),
        output_type: "searchResults",
        include_images: params.include_images.unwrap_or(linkup.include_images),
        include_domains: linkup.include_domains.as_deref(),
        exclude_domains: linkup.exclude_domains.as_deref(),
        from_date: params.from_date.as_deref().or(linkup.from_date.as_deref()),
        to_date: params.to_date.as_deref().or(linkup.to_date.as_deref()),
        max_results: params.max_results.or(linkup.max_results),
    };

    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(request_error)?
        .post(url)
        .bearer_auth(&linkup.api_key)
        .json(&request)
        .send()
        .await
        .map_err(request_error)?;

    if !response.status().is_success() {
        return Err(BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("LinkUp returned HTTP {}", response.status()),
        });
    }

    let search: SearchResponse = response.json().await.map_err(request_error)?;
    Ok(ToolResult::success_with(search))
}

fn request_error(error: reqwest::Error) -> BrowserError {
    BrowserError::ToolExecutionFailed {
        tool: TOOL_NAME.to_string(),
        reason: error.without_url().to_string(),
    }
}
