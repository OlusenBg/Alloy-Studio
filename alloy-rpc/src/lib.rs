//! `alloy-rpc` — IPC JSON-RPC protocol: message envelopes, typed events, async codec.

pub mod codec;
pub mod envelope;
pub mod error;
pub mod request;
pub mod response;
pub mod transport;
pub mod types;

// --- Top-level re-exports ----------------------------------------------------

pub use codec::{framed, framed_with_max_length, AlloyCodec, AlloyFramed};
pub use envelope::{RpcId, RpcMessage};
pub use error::{
    Error, RpcError, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND,
    PARSE_ERROR,
};
pub use request::Request;
pub use response::{Notification, Response};
pub use transport::{channel, PendingRequests, RpcReceiver, RpcSender};
pub use types::{
    BuildError, BuildErrorKind, BuildEvent, CompletionItem, Diagnostic, DiagnosticSeverity,
    DirEntry, FileEvent, FileEventKind, FileStat, Location, LogLevel, Position, Range,
    SymbolInformation, TelemetryEvent, TextEdit,
};
