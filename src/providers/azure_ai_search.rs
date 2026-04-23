use crate::config::ToolSection;
use crate::error::{BrowserError, Result};
use crate::tools::ToolResult;
use serde::{Deserialize, Serialize};

const TOOL_NAME: &str = "azure_ai_search_web";

#[derive(Debug, Clone)]
struct AzureSearchWebConfig {
    base_url: String,
    kb_name: String,
    knowledge_source_name: String,
    knowledge_source_kind: Option<String>,
    api_key: String,
}

/// Execute an Azure AI Search web request.
pub async fn execute(query: String) -> Result<ToolResult> {
    let search_config = resolve_azure_search_config().await?;

    let url = format!(
        "{}/knowledgebases/{}/retrieve?api-version=2025-11-01-preview",
        search_config.base_url.trim_end_matches('/'),
        search_config.kb_name
    );

    let request_body = serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "text": query,
                        "type": "text"
                    }
                ]
            }
        ],
        "includeActivity": false,
        "knowledgeSourceParams": [
            {
                "knowledgeSourceName": search_config.knowledge_source_name,
                "kind": search_config
                    .knowledge_source_kind
                    .as_deref()
                    .unwrap_or("web"),
                "includeReferences": true,
                "includeReferenceSourceData": true,
                "alwaysQuerySource": false
            }
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("api-key", search_config.api_key)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Failed to send request to Azure AI Search: {}", e),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!(
                "Azure AI Search request failed with status {}: {}",
                status, error_text
            ),
        });
    }

    let search_response: AzureSearchResponse = response.json().await.map_err(|e| {
        BrowserError::ToolExecutionFailed {
            tool: TOOL_NAME.to_string(),
            reason: format!("Failed to parse Azure AI Search response: {}", e),
        }
    })?;

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

    Ok(ToolResult::success_with(serde_json::json!({
        "response": response_text,
        "references": search_response.references,
    })))
}

async fn resolve_azure_search_config() -> Result<AzureSearchWebConfig> {
    let config = crate::config::get_global_config().await;
    let tool_config = match &config.tool {
        ToolSection::Search(tool_config) => tool_config,
    };

    let azure = tool_config.azure_ai_search_web.as_ref().ok_or_else(|| {
        BrowserError::ConfigError(
            "Azure AI Search Web provider is not configured in [tool.azure_ai_search_web]".to_string(),
        )
    })?;

    Ok(AzureSearchWebConfig {
        base_url: azure.base_url.clone(),
        kb_name: azure.kb_name.clone(),
        knowledge_source_name: azure.knowledge_source_name.clone(),
        knowledge_source_kind: azure.knowledge_source_kind.clone(),
        api_key: azure.api_key.clone(),
    })
}

/// Azure AI Search API response structure
#[derive(Debug, Serialize, Deserialize)]
struct AzureSearchResponse {
    #[serde(default)]
    response: Vec<ResponseItem>,
    #[serde(default)]
    activity: Vec<serde_json::Value>,
    #[serde(default)]
    references: Vec<Reference>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseItem {
    #[serde(default)]
    content: Vec<ContentItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
