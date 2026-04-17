use crate::error::{BrowserError, Result};
use crate::tools::{Tool, ToolContext, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for web search
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchParams {
    /// The search query text
    pub query: String,
}

/// Tool for performing web search using Azure AI Search
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
        // Get Azure AI Search configuration from environment variables
        let base_url = std::env::var("AZURE_AI_SEARCH_BASE_URL")
            .map_err(|_| BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: "AZURE_AI_SEARCH_BASE_URL environment variable not set".to_string(),
            })?;

        let kb_name = std::env::var("AZURE_AI_SEARCH_KB_NAME")
            .map_err(|_| BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: "AZURE_AI_SEARCH_KB_NAME environment variable not set".to_string(),
            })?;

        let knowledge_source_name = std::env::var("AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME")
            .map_err(|_| BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: "AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_NAME environment variable not set"
                    .to_string(),
            })?;

        let knowledge_source_kind = std::env::var("AZURE_AI_SEARCH_KNOWLEDGE_SOURCE_KIND")
            .unwrap_or_else(|_| "web".to_string());

        let api_key = std::env::var("AZURE_AI_SEARCH_API_KEY")
            .map_err(|_| BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: "AZURE_AI_SEARCH_API_KEY environment variable not set".to_string(),
            })?;

        // Construct the API URL
        let url = format!(
            "{}/knowledgebases/{}/retrieve?api-version=2025-11-01-preview",
            base_url.trim_end_matches('/'),
            kb_name
        );

        // Prepare the request body
        let request_body = serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "text": params.query,
                            "type": "text"
                        }
                    ]
                }
            ],
            "includeActivity": false,
            "knowledgeSourceParams": [
                {
                    "knowledgeSourceName": knowledge_source_name,
                    "kind": knowledge_source_kind,
                    "includeReferences": true,
                    "includeReferenceSourceData": true,
                    "alwaysQuerySource": false
                }
            ]
        });

        // Make the HTTP request
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("api-key", api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: format!("Failed to send request to Azure AI Search: {}", e),
            })?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: format!(
                    "Azure AI Search request failed with status {}: {}",
                    status, error_text
                ),
            });
        }

        // Parse the response
        let search_response: AzureSearchResponse = response.json().await.map_err(|e| {
            BrowserError::ToolExecutionFailed {
                tool: "web_search".to_string(),
                reason: format!("Failed to parse Azure AI Search response: {}", e),
            }
        })?;

        // Extract the response text and references
        let response_text = search_response
            .response
            .first()
            .and_then(|r| r.content.first())
            .and_then(|c| {
                if c.r#type == "text" {
                    Some(c.text.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Return the result with response text and references
        Ok(ToolResult::success_with(serde_json::json!({
            "response": response_text,
            "references": search_response.references,
        })))
    }
}

/// Azure AI Search API response structure
#[derive(Debug, Serialize, Deserialize)]
struct AzureSearchResponse {
    response: Vec<ResponseItem>,
    #[serde(default)]
    activity: Vec<serde_json::Value>,
    #[serde(default)]
    references: Vec<Reference>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseItem {
    content: Vec<ContentItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    r#type: String,
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Reference {
    #[serde(rename = "type")]
    pub ref_type: String,
    pub id: String,
    #[serde(rename = "activitySource")]
    pub activity_source: i32,
    #[serde(rename = "sourceData")]
    pub source_data: Option<serde_json::Value>,
    pub url: String,
    pub title: String,
}
