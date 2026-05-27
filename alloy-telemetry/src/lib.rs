//! `alloy-telemetry` — Live robot telemetry: TCP listener, protocol decoder,
//! broadcast stream, and ring-buffer history.

pub mod error;
pub mod history;
pub mod protocol;
pub mod server;
pub mod stream;

// Top-level re-exports
pub use error::TelemetryError;
pub use history::{HistoryBuffer, PacketRecord, DEFAULT_HISTORY_CAPACITY};
pub use protocol::{Packet, PacketCodec, PacketKind, TelemetryFramed, MAX_PACKET_BYTES};
pub use server::TelemetryServer;
pub use stream::{TelemetryStream, BROADCAST_CAPACITY};
