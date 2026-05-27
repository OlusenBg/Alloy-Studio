//! Error types for the alloy-proxy crate.

/// All errors that can occur in the proxy subsystem.
#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("watch error: {0}")]
    Watch(String),

    #[error("channel closed")]
    ChannelClosed,
}
