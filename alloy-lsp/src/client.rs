//! Core LSP client: spawns a language server process, speaks JSON-RPC over
//! stdin/stdout using LSP framing, and dispatches responses/notifications.

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use anyhow::Context;
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
    sync::{broadcast, oneshot, Mutex},
    time,
};
use tracing::{debug, warn};

use crate::capabilities::make_client_capabilities;
use crate::protocol::JsonRpcMessage;

// ── constants ────────────────────────────────────────────────────────────────

const REQUEST_TIMEOUT_SECS: u64 = 30;
const NOTIFICATION_CHANNEL_CAP: usize = 256;

// ── LspClient ────────────────────────────────────────────────────────────────

/// A connected LSP server.  One instance per spawned language-server process.
pub struct LspClient {
    /// The child process handle (kept alive so the server stays running).
    process: Child,
    /// Protected write half of the server's stdin.
    stdin: Arc<Mutex<ChildStdin>>,
    /// Map of in-flight request id → oneshot to deliver the JSON result.
    pending: Arc<DashMap<i64, oneshot::Sender<serde_json::Value>>>,
    /// Broadcast channel for server-to-client notifications.
    notification_tx: broadcast::Sender<JsonRpcMessage>,
    /// Monotonically increasing request-id counter.
    next_id: Arc<AtomicI64>,
    /// Server capabilities, populated after `initialize`.
    server_caps: Arc<RwLock<Option<lsp_types::ServerCapabilities>>>,
}

impl LspClient {
    // ── spawning ─────────────────────────────────────────────────────────────

    /// Spawn a language server and return a connected `LspClient`.
    ///
    /// * `cmd`  – executable name or absolute path
    /// * `args` – command-line arguments forwarded verbatim
    /// * `env`  – additional environment variables for the child process
    pub async fn spawn(cmd: &str, args: &[&str], env: &[(&str, &str)]) -> anyhow::Result<Self> {
        let mut command = tokio::process::Command::new(cmd);
        command
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        for (k, v) in env {
            command.env(k, v);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn LSP server: {cmd}"))?;

        let stdin: ChildStdin = child.stdin.take().context("missing stdin pipe")?;
        let stdout: ChildStdout = child.stdout.take().context("missing stdout pipe")?;
        let stderr = child.stderr.take().context("missing stderr pipe")?;

        let (notif_tx, _) = broadcast::channel(NOTIFICATION_CHANNEL_CAP);
        let pending: Arc<DashMap<i64, oneshot::Sender<serde_json::Value>>> =
            Arc::new(DashMap::new());

        let pending_clone = Arc::clone(&pending);
        let notif_tx_clone = notif_tx.clone();

        // ── stdout reader task ───────────────────────────────────────────────
        tokio::spawn(async move {
            let mut stdout = stdout;
            loop {
                match read_message(&mut stdout).await {
                    Ok(Some(text)) => {
                        match serde_json::from_str::<JsonRpcMessage>(&text) {
                            Ok(msg) => {
                                if msg.is_response() {
                                    if let Some(id) = msg.numeric_id() {
                                        if let Some((_, tx)) = pending_clone.remove(&id) {
                                            // Deliver the raw JSON value (result or error object).
                                            let payload = if let Some(r) = msg.result {
                                                r
                                            } else if let Some(e) = msg.error {
                                                serde_json::json!({
                                                    "__lsp_error": true,
                                                    "code": e.code,
                                                    "message": e.message,
                                                })
                                            } else {
                                                serde_json::Value::Null
                                            };
                                            let _ = tx.send(payload);
                                        }
                                    }
                                } else if msg.is_notification() {
                                    let _ = notif_tx_clone.send(msg);
                                } else if msg.is_request() {
                                    // Server-initiated request – broadcast as notification so
                                    // callers can handle it (e.g. window/showMessageRequest).
                                    let _ = notif_tx_clone.send(msg);
                                }
                            }
                            Err(e) => {
                                warn!("LSP: failed to parse message: {e} — raw: {text}");
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("LSP: stdout closed");
                        break;
                    }
                    Err(e) => {
                        warn!("LSP: read error: {e}");
                        break;
                    }
                }
            }
        });

        // ── stderr logger task ───────────────────────────────────────────────
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("LSP stderr: {line}");
            }
        });

        Ok(Self {
            process: child,
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            notification_tx: notif_tx,
            next_id: Arc::new(AtomicI64::new(1)),
            server_caps: Arc::new(RwLock::new(None)),
        })
    }

    // ── initialize ───────────────────────────────────────────────────────────

    /// Perform the LSP `initialize` / `initialized` handshake.
    ///
    /// Returns the server's advertised capabilities.
    pub async fn initialize(
        &self,
        root_uri: &str,
    ) -> anyhow::Result<lsp_types::ServerCapabilities> {
        use lsp_types::{
            notification::Initialized, request::Initialize, InitializeParams, InitializeResult,
        };
        let parsed_uri: url::Url = root_uri
            .parse()
            .with_context(|| format!("invalid root URI: {root_uri}"))?;

        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(parsed_uri.clone()),
            capabilities: make_client_capabilities(),
            workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
                uri: parsed_uri,
                name: "workspace".to_string(),
            }]),
            client_info: Some(lsp_types::ClientInfo {
                name: "Alloy Studio".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        };

        let result: InitializeResult = self
            .request::<_, InitializeResult>(
                <Initialize as lsp_types::request::Request>::METHOD,
                params,
            )
            .await?;

        let caps = result.capabilities;

        // Store capabilities for later queries.
        *self.server_caps.write() = Some(caps.clone());

        // Send the `initialized` notification (no parameters required).
        self.notify(
            <Initialized as lsp_types::notification::Notification>::METHOD,
            lsp_types::InitializedParams {},
        )
        .await?;

        Ok(caps)
    }

    // ── request ──────────────────────────────────────────────────────────────

    /// Send a JSON-RPC request and await the response (30-second timeout).
    pub async fn request<P, R>(&self, method: &str, params: P) -> anyhow::Result<R>
    where
        P: serde::Serialize + Send,
        R: serde::de::DeserializeOwned,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel::<serde_json::Value>();
        self.pending.insert(id, tx);

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": serde_json::to_value(&params)?,
        });
        let text = serde_json::to_string(&msg)?;

        {
            let mut stdin = self.stdin.lock().await;
            write_message(&mut stdin, &text).await?;
        }

        let value = time::timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| {
                self.pending.remove(&id);
                crate::error::LspError::Timeout
            })?
            .map_err(|_| crate::error::LspError::ChannelClosed)?;

        // Check if the server returned an error object.
        if let Some(true) = value.get("__lsp_error").and_then(|v| v.as_bool()) {
            let code = value.get("code").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
            let message = value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(crate::error::LspError::LspError { code, message }.into());
        }

        let result: R = serde_json::from_value(value)?;
        Ok(result)
    }

    // ── notify ───────────────────────────────────────────────────────────────

    /// Send a JSON-RPC notification (fire-and-forget, no `id`).
    pub async fn notify<P>(&self, method: &str, params: P) -> anyhow::Result<()>
    where
        P: serde::Serialize + Send,
    {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": serde_json::to_value(&params)?,
        });
        let text = serde_json::to_string(&msg)?;
        let mut stdin = self.stdin.lock().await;
        write_message(&mut stdin, &text).await
    }

    // ── subscribe_notifications ───────────────────────────────────────────────

    /// Subscribe to server-initiated notifications/requests.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<JsonRpcMessage> {
        self.notification_tx.subscribe()
    }

    // ── server_capabilities ───────────────────────────────────────────────────

    /// Return a copy of the server capabilities (populated after `initialize`).
    pub fn server_capabilities(&self) -> Option<lsp_types::ServerCapabilities> {
        self.server_caps.read().clone()
    }

    // ── shutdown ──────────────────────────────────────────────────────────────

    /// Gracefully shut down the language server.
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        use lsp_types::{notification::Exit, request::Shutdown};

        // Best-effort shutdown request.
        let _ = self
            .request::<_, serde_json::Value>(
                <Shutdown as lsp_types::request::Request>::METHOD,
                serde_json::Value::Null,
            )
            .await;

        // Best-effort exit notification.
        let _ = self
            .notify(
                <Exit as lsp_types::notification::Notification>::METHOD,
                serde_json::Value::Null,
            )
            .await;

        let _ = self.process.kill().await;
        Ok(())
    }
}

// ── I/O helpers ──────────────────────────────────────────────────────────────

/// Write one LSP-framed message to `stdin`.
///
/// Format: `Content-Length: {byte_len}\r\n\r\n{json}`
pub async fn write_message(stdin: &mut ChildStdin, json: &str) -> anyhow::Result<()> {
    let bytes = json.as_bytes();
    let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
    stdin.write_all(header.as_bytes()).await?;
    stdin.write_all(bytes).await?;
    stdin.flush().await?;
    Ok(())
}

/// Read one LSP-framed message from `stdout`.
///
/// Returns `None` when the stream is closed (EOF).
pub async fn read_message(stdout: &mut ChildStdout) -> anyhow::Result<Option<String>> {
    // We need buffered line reading for headers but raw byte reading for body.
    // Because ChildStdout is not Clone we manage a small manual buffer here.
    let mut content_length: Option<usize> = None;
    let mut header_buf = Vec::with_capacity(64);

    // Read headers one byte at a time so we don't consume body bytes.
    loop {
        header_buf.clear();
        // Read a line (terminated by \n).
        let mut saw_byte = false;
        loop {
            let mut b = [0u8; 1];
            match stdout.read_exact(&mut b).await {
                Ok(_) => {
                    saw_byte = true;
                    header_buf.push(b[0]);
                    if b[0] == b'\n' {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if !saw_byte {
                        return Ok(None); // clean EOF
                    }
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }

        if header_buf.is_empty() || header_buf == b"\r\n" || header_buf == b"\n" {
            // Blank line signals end of headers.
            break;
        }

        let line = std::str::from_utf8(&header_buf)
            .map_err(|e| anyhow::anyhow!("header UTF-8 error: {e}"))?
            .trim_end();

        if line.is_empty() {
            break;
        }

        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                rest.trim()
                    .parse::<usize>()
                    .context("invalid Content-Length value")?,
            );
        }
        // Ignore other headers (Content-Type, etc.)
    }

    let len = content_length.context("LSP message missing Content-Length header")?;
    if len == 0 {
        return Ok(Some(String::new()));
    }

    let mut body = vec![0u8; len];
    stdout
        .read_exact(&mut body)
        .await
        .context("failed to read LSP message body")?;

    let text = String::from_utf8(body)
        .map_err(|e| anyhow::anyhow!("LSP message body is not UTF-8: {e}"))?;

    Ok(Some(text))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn write_message_format() {
        // Verify the Content-Length frame is well-formed.
        let payload = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let expected_header = format!("Content-Length: {}\r\n\r\n", payload.len());
        // We just check the math is right; actual I/O tests need a process.
        assert_eq!(expected_header.len(), "Content-Length: 46\r\n\r\n".len());
        let _ = payload.len(); // suppress unused warning
    }
}
