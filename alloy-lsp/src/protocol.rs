//! JSON-RPC helpers for LSP communication.

use std::sync::atomic::{AtomicI64, Ordering};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ID counter
// ---------------------------------------------------------------------------

/// Global atomic counter for JSON-RPC request IDs.
static NEXT_ID: AtomicI64 = AtomicI64::new(1);

/// Allocate a unique JSON-RPC request ID.
pub fn next_id() -> i64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Message construction
// ---------------------------------------------------------------------------

/// Build a JSON-RPC request object for the given LSP request type.
pub fn make_request<R: lsp_types::request::Request>(
    id: i64,
    params: R::Params,
) -> serde_json::Value
where
    R::Params: Serialize,
{
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": R::METHOD,
        "params": serde_json::to_value(params).unwrap_or(serde_json::Value::Null),
    })
}

/// Build a JSON-RPC notification object for the given LSP notification type.
pub fn make_notification<N: lsp_types::notification::Notification>(
    params: N::Params,
) -> serde_json::Value
where
    N::Params: Serialize,
{
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": N::METHOD,
        "params": serde_json::to_value(params).unwrap_or(serde_json::Value::Null),
    })
}

/// Parse a JSON-RPC response into the expected result type for an LSP request.
pub fn parse_response<R: lsp_types::request::Request>(
    value: serde_json::Value,
) -> anyhow::Result<R::Result>
where
    R::Result: serde::de::DeserializeOwned,
{
    let msg: JsonRpcMessage = serde_json::from_value(value)?;

    if let Some(err) = msg.error {
        anyhow::bail!("LSP error {}: {}", err.code, err.message);
    }

    let result = msg
        .result
        .ok_or_else(|| anyhow::anyhow!("JSON-RPC response missing 'result' field"))?;

    let parsed: R::Result = serde_json::from_value(result)?;
    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

/// A JSON-RPC 2.0 message, covering requests, responses, and notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcMessage {
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

    /// Extract the numeric ID, if present and numeric.
    pub fn numeric_id(&self) -> Option<i64> {
        self.id.as_ref().and_then(|v| v.as_i64())
    }
}
