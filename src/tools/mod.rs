//! Web search tools module
//!
//! This module provides tools for web search operations.

pub mod web_search;

// Re-export Params types for use by MCP layer
pub use web_search::WebSearchParams;

use crate::error::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Tool execution context
pub struct ToolContext<'a> {
    /// Phantom data to maintain lifetime
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> ToolContext<'a> {
    /// Create a new tool context
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> Default for ToolContext<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of tool execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    /// Whether the tool execution was successful
    pub success: bool,

    /// Result data (JSON value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Error message if execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, Value>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(data: Option<Value>) -> Self {
        Self {
            success: true,
            data,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a successful result with data
    pub fn success_with<T: serde::Serialize>(data: T) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).ok(),
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a failure result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the result
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Trait for browser automation tools with associated parameter types
#[async_trait::async_trait]
pub trait Tool: Send + Sync + Default {
    /// Associated parameter type for this tool
    type Params: serde::Serialize + for<'de> serde::Deserialize<'de> + schemars::JsonSchema + Send;

    /// Get tool name
    fn name(&self) -> &str;

    /// Get tool parameter schema (JSON Schema)
    fn parameters_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(Self::Params)).unwrap_or_default()
    }

    /// Execute the tool with strongly-typed parameters
    async fn execute_typed(&self, params: Self::Params, context: &mut ToolContext<'_>) -> Result<ToolResult>;

    /// Execute the tool with JSON parameters (default implementation)
    async fn execute(&self, params: Value, context: &mut ToolContext<'_>) -> Result<ToolResult> {
        let typed_params: Self::Params = serde_json::from_value(params).map_err(|e| {
            crate::error::BrowserError::InvalidArgument(format!("Invalid parameters: {}", e))
        })?;
        self.execute_typed(typed_params, context).await
    }
}

/// Type-erased tool trait for dynamic dispatch
#[async_trait::async_trait]
pub trait DynTool: Send + Sync {
    fn name(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, params: Value, context: &mut ToolContext<'_>) -> Result<ToolResult>;
}

/// Blanket implementation to convert any Tool into DynTool
#[async_trait::async_trait]
impl<T: Tool> DynTool for T {
    fn name(&self) -> &str {
        Tool::name(self)
    }

    fn parameters_schema(&self) -> Value {
        Tool::parameters_schema(self)
    }

    async fn execute(&self, params: Value, context: &mut ToolContext<'_>) -> Result<ToolResult> {
        Tool::execute(self, params, context).await
    }
}

/// Tool registry for managing and accessing tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn DynTool>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with default tools
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register the web_search tool
        registry.register(web_search::WebSearchTool);

        registry
    }

    /// Register a tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn DynTool>> {
        self.tools.get(name)
    }

    /// Check if a tool exists
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List all tool names
    pub fn list_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all tools
    pub fn all_tools(&self) -> Vec<Arc<dyn DynTool>> {
        self.tools.values().cloned().collect()
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        params: Value,
        context: &mut ToolContext<'_>,
    ) -> Result<ToolResult> {
        match self.get(name) {
            Some(tool) => tool.execute(params, context).await,
            None => Ok(ToolResult::failure(format!("Tool '{}' not found", name))),
        }
    }

    /// Get the number of registered tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(Some(serde_json::json!({"url": "https://example.com"})));
        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult::failure("Test error");
        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_tool_result_with_metadata() {
        let result = ToolResult::success(None).with_metadata("duration_ms", serde_json::json!(100));

        assert!(result.metadata.contains_key("duration_ms"));
    }
}
