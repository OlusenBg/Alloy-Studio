//! Error types for the telemetry subsystem.

/// Errors produced by the telemetry protocol and server.
#[derive(thiserror::Error, Debug)]
pub enum TelemetryError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid packet length: {0}")]
    InvalidLength(u32),

    #[error("connection closed")]
    ConnectionClosed,
}
