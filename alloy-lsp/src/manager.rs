//! LSP server manager: tracks running server instances keyed by language ID,
//! starts new servers on demand, and shuts them all down cleanly.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use which::which;

use alloy_core::config::AlloyConfig;

use crate::client::LspClient;

// ── ServerHandle ─────────────────────────────────────────────────────────────

/// A live language-server connection plus associated metadata.
pub struct ServerHandle {
    /// The connected client (wrapped for shared async access).
    pub client: Arc<Mutex<LspClient>>,
    /// LSP language identifier (e.g. `"java"`, `"kotlin"`).
    pub language_id: String,
    /// Capabilities reported by the server during `initialize`.
    pub capabilities: Option<lsp_types::ServerCapabilities>,
}

// ── LspManager ───────────────────────────────────────────────────────────────

/// Manages zero or more running language servers for a single workspace.
#[allow(dead_code)]
pub struct LspManager {
    /// language-id → live server handle.
    servers: DashMap<String, ServerHandle>,
    /// Absolute path of the workspace root used as `rootUri`.
    workspace_root: PathBuf,
    /// Editor-level configuration (read-locked on access).
    config: Arc<RwLock<AlloyConfig>>,
}

impl LspManager {
    /// Create a new manager.  No servers are started until `get_or_start` is called.
    pub fn new(workspace_root: PathBuf, config: Arc<RwLock<AlloyConfig>>) -> Self {
        Self {
            servers: DashMap::new(),
            workspace_root,
            config,
        }
    }

    /// Return the client for `language_id`, starting a new server if necessary.
    pub async fn get_or_start(&self, language_id: &str) -> anyhow::Result<Arc<Mutex<LspClient>>> {
        // Fast path: server already running.
        if let Some(handle) = self.servers.get(language_id) {
            return Ok(Arc::clone(&handle.client));
        }

        // Determine launch command.
        let (cmd, args) = Self::server_command(language_id)
            .ok_or_else(|| crate::error::LspError::ServerNotFound(language_id.to_string()))?;

        info!("LSP: starting server for language '{language_id}': {cmd}");

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let client = LspClient::spawn(&cmd, &arg_refs, &[])
            .await
            .with_context(|| format!("failed to spawn LSP server for language '{language_id}'"))?;

        // Build root URI from workspace path.
        let root_uri = url::Url::from_file_path(&self.workspace_root)
            .map_err(|()| anyhow::anyhow!("workspace root is not an absolute path"))?
            .to_string();

        let caps = client
            .initialize(&root_uri)
            .await
            .with_context(|| format!("LSP initialize failed for language '{language_id}'"))?;

        debug!("LSP: server for '{language_id}' initialized, capabilities: {caps:?}");

        let client_arc = Arc::new(Mutex::new(client));

        self.servers.insert(
            language_id.to_string(),
            ServerHandle {
                client: Arc::clone(&client_arc),
                language_id: language_id.to_string(),
                capabilities: Some(caps),
            },
        );

        Ok(client_arc)
    }

    /// Shut down all running servers gracefully.
    pub async fn shutdown_all(&self) {
        let keys: Vec<String> = self.servers.iter().map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some((_, handle)) = self.servers.remove(&key) {
                let mut client = handle.client.lock().await;
                if let Err(e) = client.shutdown().await {
                    warn!(
                        "LSP: error shutting down server for '{}': {e}",
                        handle.language_id
                    );
                }
            }
        }
    }

    /// Map a language ID to a server launch command + arguments.
    ///
    /// Returns `None` if no server is available for that language.
    pub fn server_command(language_id: &str) -> Option<(String, Vec<String>)> {
        match language_id {
            "java" | "kotlin" => {
                // Prefer `jdtls` on PATH.
                if let Ok(path) = which("jdtls") {
                    let cmd = path.to_string_lossy().into_owned();
                    return Some((cmd, vec![]));
                }
                // Fallback: bare name (let the OS find it).
                Some(("jdtls".to_string(), vec![]))
            }
            // Groovy has no usable standalone LSP today.
            "groovy" => None,
            // Unknown language.
            _ => None,
        }
    }
}
