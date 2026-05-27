//! Core error types for the alloy-core crate.

/// The primary error type for alloy-core operations.
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("buffer: {0}")]
    Buffer(String),

    #[error("syntax: {0}")]
    Syntax(String),

    #[error("config: {0}")]
    Config(String),

    #[error("search: {0}")]
    Search(String),

    #[error("document not found: {uri}")]
    DocumentNotFound { uri: String },

    #[error("invalid position: line {line} col {col}")]
    InvalidPosition { line: u32, col: u32 },
}

/// Convenience `Result` alias for alloy-core operations.
pub type Result<T> = std::result::Result<T, CoreError>;
