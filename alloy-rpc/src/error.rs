//! RPC error types and standard JSON-RPC error codes.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Standard JSON-RPC error codes
// ---------------------------------------------------------------------------

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

// ---------------------------------------------------------------------------
// RpcError — wire-level JSON-RPC error object
// ---------------------------------------------------------------------------

/// A JSON-RPC error object that can be embedded in a response message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    /// Create an `RpcError` with an explicit code and message.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Attach arbitrary data to this error.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    // --- Standard constructors -----------------------------------------------

    pub fn parse_error() -> Self {
        Self::new(PARSE_ERROR, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(INVALID_REQUEST, "Invalid Request")
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            METHOD_NOT_FOUND,
            format!("Method not found: {method}"),
        )
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, msg)
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

// ---------------------------------------------------------------------------
// Error — crate-level error enum
// ---------------------------------------------------------------------------

/// Crate-level error type for `alloy-rpc`.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("rpc: {0}")]
    Rpc(RpcError),

    #[error("channel closed")]
    ChannelClosed,
}

impl From<RpcError> for Error {
    fn from(e: RpcError) -> Self {
        Error::Rpc(e)
    }
}
