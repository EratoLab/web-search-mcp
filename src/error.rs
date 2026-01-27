use thiserror::Error;

/// Core error type for web search operations
#[derive(Error, Debug)]
pub enum BrowserError {
    /// Tool execution failed
    #[error("Tool '{tool}' execution failed: {reason}")]
    ToolExecutionFailed { tool: String, reason: String },

    /// Invalid argument provided to a function
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type alias for web search operations
pub type Result<T> = std::result::Result<T, BrowserError>;
