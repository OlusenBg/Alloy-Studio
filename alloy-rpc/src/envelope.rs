//! Raw JSON-RPC 2.0 message envelope types.

use serde::{Deserialize, Serialize};

use crate::error::RpcError;

// ---------------------------------------------------------------------------
// RpcId
// ---------------------------------------------------------------------------

/// A JSON-RPC request/response identifier — either a number or a string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    Number(i64),
    Str(String),
}

impl RpcId {
    /// Return the inner number, or `None` if this is a string id.
    pub fn as_number(&self) -> Option<i64> {
        match self {
            RpcId::Number(n) => Some(*n),
            RpcId::Str(_) => None,
        }
    }

    /// Return the inner string, or `None` if this is a number id.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RpcId::Str(s) => Some(s.as_str()),
            RpcId::Number(_) => None,
        }
    }

    /// Consume this id and return the number, panicking if it is a string.
    pub fn into_number(self) -> i64 {
        match self {
            RpcId::Number(n) => n,
            RpcId::Str(s) => panic!("expected Number id, got Str({s:?})"),
        }
    }

    /// Consume this id and return the string, panicking if it is a number.
    pub fn into_str(self) -> String {
        match self {
            RpcId::Str(s) => s,
            RpcId::Number(n) => panic!("expected Str id, got Number({n})"),
        }
    }
}

impl From<i64> for RpcId {
    fn from(n: i64) -> Self {
        RpcId::Number(n)
    }
}

impl From<String> for RpcId {
    fn from(s: String) -> Self {
        RpcId::Str(s)
    }
}

impl From<&str> for RpcId {
    fn from(s: &str) -> Self {
        RpcId::Str(s.to_owned())
    }
}

impl std::fmt::Display for RpcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcId::Number(n) => write!(f, "{n}"),
            RpcId::Str(s) => write!(f, "{s}"),
        }
    }
}

// ---------------------------------------------------------------------------
// RpcMessage
// ---------------------------------------------------------------------------

/// A raw JSON-RPC 2.0 message.
///
/// Depending on which fields are set the message represents:
/// - **request**: `id` + `method` + optional `params`
/// - **notification**: no `id`, `method` + optional `params`
/// - **success response**: `id` + `result`
/// - **error response**: `id` + `error`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcMessage {
    pub jsonrpc: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RpcId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcMessage {
    /// The JSON-RPC version string used in all messages.
    pub const VERSION: &'static str = "2.0";

    // --- Constructors --------------------------------------------------------

    /// Build a request message (has an `id`, a `method`, and optional params).
    pub fn new_request(id: RpcId, method: impl Into<String>, params: impl Serialize) -> Self {
        let params_value = serde_json::to_value(params).unwrap_or(serde_json::Value::Null);
        let params = if params_value.is_null() {
            None
        } else {
            Some(params_value)
        };

        Self {
            jsonrpc: Self::VERSION.to_owned(),
            id: Some(id),
            method: Some(method.into()),
            params,
            result: None,
            error: None,
        }
    }

    /// Build a notification message (no `id`).
    pub fn new_notification(method: impl Into<String>, params: impl Serialize) -> Self {
        let params_value = serde_json::to_value(params).unwrap_or(serde_json::Value::Null);
        let params = if params_value.is_null() {
            None
        } else {
            Some(params_value)
        };

        Self {
            jsonrpc: Self::VERSION.to_owned(),
            id: None,
            method: Some(method.into()),
            params,
            result: None,
            error: None,
        }
    }

    /// Build a successful response message.
    pub fn new_ok(id: RpcId, result: impl Serialize) -> Self {
        let result_value = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);

        Self {
            jsonrpc: Self::VERSION.to_owned(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(result_value),
            error: None,
        }
    }

    /// Build an error response message.
    pub fn new_err(id: RpcId, error: RpcError) -> Self {
        Self {
            jsonrpc: Self::VERSION.to_owned(),
            id: Some(id),
            method: None,
            params: None,
            result: None,
            error: Some(error),
        }
    }

    // --- Classification ------------------------------------------------------

    /// Returns `true` when this message is a request (has both `id` and `method`).
    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    /// Returns `true` when this message is a response (has `id` but no `method`).
    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }

    /// Returns `true` when this message is a notification (has `method` but no `id`).
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }
}
