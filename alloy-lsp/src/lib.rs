//! `alloy-lsp` — LSP client: spawns language servers, manages capability
//! negotiation, and provides FTC/JDTLS configuration helpers.

pub mod capabilities;
pub mod client;
pub mod error;
pub mod ftc_jdtls;
pub mod manager;
pub mod protocol;

pub use capabilities::{
    make_client_capabilities, supports_completion, supports_formatting, supports_goto_definition,
    supports_hover, text_document_sync_kind,
};
pub use client::LspClient;
pub use error::LspError;
pub use ftc_jdtls::{FtcJdtlsConfig, FtcSdkLocator};
pub use manager::{LspManager, ServerHandle};
pub use protocol::{JsonRpcError, JsonRpcMessage, make_notification, make_request, next_id, parse_response};
