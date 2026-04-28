use crate::config::ToolSection;
use crate::error::{BrowserError, Result};
use crate::tools::ToolResult;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Url};
use serde_json::{Map, Value, json};
use std::time::Duration;

const TOOL_NAME: &str = "genai_responses";
const DEFAULT_SYSTEM_PROMPT: &str =
    "Use the web_search tool and return a concise answer with source-backed details.";

#[derive(Debug, Clone)]
struct GenAIResponsesRuntimeConfig {
    response_endpoint: String,
    model: String,
    api_key: String,
    api_key_header: String,
    system_prompt: Option<String>,
    reasoning: String,
    tool_max_uses: Option<u32>,
    tool_allowed_domains: Option<Vec<String>>,
    tool_blocked_domains: Option<Vec<String>>,
}

pub async fn execute(query: String) -> Result<ToolResult> {
    let config = resolve_genai_responses_config().await?;

    let client = build_client()?;
    let request_headers = build_request_headers(&config)?;
    let request_payload = build_request_payload(&query, &config);

    let response = client
        .post(&config.response_endpoint)
        .headers(request_headers)
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Failed to send request to Responses API: {e}"),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Responses API request failed with status {status}: {error_text}"),
        });
    }

    let response_body: Value =
        response
            .json()
            .await
            .map_err(|e| BrowserError::ToolExecutionFailed {
                tool: TOOL_NAME.to_string(),
                reason: format!("Failed to parse Responses API response: {e}"),
            })?;

    let response_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let response_model = response_body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(&config.model)
        .to_string();
    let response_text = extract_response_text(&response_body)
        .unwrap_or_else(|| "No textual response returned by Responses API provider".to_string());
    let sources = extract_sources(&response_body);

    Ok(ToolResult::success_with(serde_json::json!({
        "query": query,
        "text": response_text,
        "model": response_model,
        "sources": sources,
        "response_id": response_id,
    })))
}

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Failed to build Responses API reqwest client: {e}"),
        })
}

fn build_request_headers(config: &GenAIResponsesRuntimeConfig) -> Result<HeaderMap> {
    let is_authorization = config
        .api_key_header
        .trim()
        .eq_ignore_ascii_case("authorization")
        || config
            .api_key_header
            .trim()
            .eq_ignore_ascii_case("authorization-token");

    let header_name = HeaderName::from_bytes(config.api_key_header.as_bytes()).map_err(|e| {
        BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!(
                "Invalid API key header name '{}': {e}",
                config.api_key_header
            ),
        }
    })?;

    let mut headers = HeaderMap::new();
    let header_value = if is_authorization {
        format!("Bearer {}", config.api_key)
    } else {
        config.api_key.clone()
    };

    headers.insert(
        header_name,
        HeaderValue::from_str(&header_value).map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!(
                "Invalid API key header value for '{}': {e}",
                config.api_key_header
            ),
        })?,
    );

    Ok(headers)
}

fn build_request_payload(query: &str, config: &GenAIResponsesRuntimeConfig) -> Value {
    let mut web_search_tool = Map::new();
    web_search_tool.insert("type".to_string(), Value::String("web_search".to_string()));

    if let Some(max_uses) = config.tool_max_uses {
        web_search_tool.insert("max_uses".to_string(), json!(max_uses));
    }
    if let Some(allowed_domains) = &config.tool_allowed_domains {
        web_search_tool.insert("allowed_domains".to_string(), json!(allowed_domains));
    }
    if let Some(blocked_domains) = &config.tool_blocked_domains {
        web_search_tool.insert("blocked_domains".to_string(), json!(blocked_domains));
    }

    json!({
        "input": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": query,
                    }
                ],
            }
        ],
        "reasoning": {
            "effort": config.reasoning,
        },
        "instructions": config.system_prompt.as_deref().unwrap_or(DEFAULT_SYSTEM_PROMPT),
        "model": config.model,
        "tools": [Value::Object(web_search_tool)],
        "include": ["web_search_call.action.sources"],
        "tool_choice": "required"
    })
}

fn extract_response_text(response_body: &Value) -> Option<String> {
    let output = response_body.get("output")?.as_array()?;

    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }

        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };

        let mut chunks: Vec<String> = Vec::new();
        for block in content {
            if block.get("type").and_then(Value::as_str) == Some("output_text") {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    chunks.push(text.to_string());
                }
            }
        }

        if !chunks.is_empty() {
            return Some(chunks.join("\n"));
        }
    }

    None
}

fn extract_sources(response_body: &Value) -> Option<Value> {
    let output = response_body.get("output")?.as_array()?;

    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("web_search_call") {
            continue;
        }

        if let Some(sources) = item
            .get("action")
            .and_then(|action| action.get("sources"))
            .cloned()
        {
            return Some(sources);
        }
    }

    None
}

fn build_endpoints(base_url: &str, api_version: Option<&str>) -> Result<String> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err(BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: "base_url for genai_responses is empty".to_string(),
        });
    }

    let mut normalized_base_url = base_url.to_string();
    if !normalized_base_url.ends_with('/') {
        normalized_base_url.push('/');
    }

    let mut endpoint =
        Url::parse(&normalized_base_url).map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Invalid genai_responses base_url '{base_url}': {e}"),
        })?;

    if let Some(api_version) = api_version.map(str::trim).filter(|v| !v.is_empty()) {
        endpoint
            .query_pairs_mut()
            .append_pair("api-version", api_version);
    }

    let response_endpoint = {
        let query = endpoint.query().map(str::to_string);
        let mut response_url = if endpoint.path().ends_with("/responses") {
            endpoint.clone()
        } else {
            endpoint.join("responses").map_err(|e| BrowserError::ToolExecutionFailed {
                tool: TOOL_NAME.to_string(),
                reason: format!(
                    "Failed to build responses URL from genai_responses base_url '{base_url}': {e}"
                ),
            })?
        };
        if let Some(query) = query {
            response_url.set_query(Some(&query));
        }
        response_url.to_string()
    };

    Ok(response_endpoint)
}

async fn resolve_genai_responses_config() -> Result<GenAIResponsesRuntimeConfig> {
    let config = crate::config::get_global_config().await;
    let tool_config = match &config.tool {
        ToolSection::Search(tool_config) => tool_config,
    };

    let genai_config = tool_config.genai_responses.as_ref().ok_or_else(|| {
        BrowserError::ConfigError(
            "GenAI Responses provider is not configured in [tool.genai_responses]".to_string(),
        )
    })?;

    let response_endpoint =
        build_endpoints(&genai_config.base_url, genai_config.api_version.as_deref())?;

    Ok(GenAIResponsesRuntimeConfig {
        response_endpoint,
        model: genai_config.model.clone(),
        api_key: genai_config.api_key.clone(),
        api_key_header: genai_config
            .api_key_header
            .clone()
            .unwrap_or_else(|| "Authorization".to_string()),
        system_prompt: genai_config.system_prompt.clone(),
        reasoning: genai_config
            .reasoning
            .clone()
            .unwrap_or_else(|| "none".to_string()),
        tool_max_uses: genai_config.tool_max_uses,
        tool_allowed_domains: genai_config.tool_allowed_domains.clone(),
        tool_blocked_domains: genai_config.tool_blocked_domains.clone(),
    })
}
