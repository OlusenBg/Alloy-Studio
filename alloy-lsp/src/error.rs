//! Error types for the alloy-lsp crate.

/// All errors that can occur in the LSP subsystem.
#[derive(thiserror::Error, Debug)]
pub enum LspError {
    #[error("server not found for language: {0}")]
    ServerNotFound(String),

    #[error("server process error: {0}")]
    Process(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("server not initialized")]
    NotInitialized,

    #[error("request timeout")]
    Timeout,

    #[error("channel closed")]
    ChannelClosed,

    #[error("lsp error {code}: {message}")]
    LspError { code: i32, message: String },
}
