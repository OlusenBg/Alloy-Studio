//! IPC server: accepts connections over TCP (or a Unix socket on supported
//! platforms) and dispatches JSON-RPC messages to the request handler.

use std::net::SocketAddr;
use std::sync::Arc;

use alloy_rpc::codec::AlloyCodec;
use alloy_rpc::envelope::{RpcId, RpcMessage};
use alloy_rpc::response::{Notification, Response};
use alloy_rpc::types::FileEvent;
use crossbeam_channel::Receiver;
use futures::{SinkExt, StreamExt};
use tokio_util::codec::Framed;
use tracing::{debug, info, warn};

use crate::handler::RequestHandler;

// ── ProxyServer ───────────────────────────────────────────────────────────────

/// Binds a network listener and dispatches connections to the request handler.
pub struct ProxyServer {
    handler: Arc<RequestHandler>,
    file_event_rx: Receiver<FileEvent>,
}

impl ProxyServer {
    /// Create a new `ProxyServer`.
    pub fn new() -> anyhow::Result<Self> {
        let (handler, file_event_rx) = RequestHandler::new()?;
        Ok(Self {
            handler: Arc::new(handler),
            file_event_rx,
        })
    }

    /// Accept connections on a TCP socket.
    pub async fn listen_tcp(self: Arc<Self>, addr: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("proxy IPC server listening on tcp://{addr}");

        loop {
            let (stream, peer) = listener.accept().await?;
            let peer_str = peer.to_string();
            info!("proxy: new TCP connection from {peer_str}");

            let handler = Arc::clone(&self.handler);
            let event_rx = self.file_event_rx.clone();

            tokio::spawn(async move {
                Self::handle_connection(handler, stream, peer_str, event_rx).await;
            });
        }
    }

    /// Accept connections on a Unix domain socket (Unix targets only).
    #[cfg(unix)]
    pub async fn listen_unix(self: Arc<Self>, path: &std::path::Path) -> anyhow::Result<()> {
        // Remove a stale socket file if present.
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = tokio::net::UnixListener::bind(path)?;
        info!("proxy IPC server listening on unix://{}", path.display());

        loop {
            let (stream, _peer) = listener.accept().await?;
            info!("proxy: new Unix connection");

            let handler = Arc::clone(&self.handler);
            let event_rx = self.file_event_rx.clone();

            tokio::spawn(async move {
                Self::handle_connection(handler, stream, "unix".to_string(), event_rx).await;
            });
        }
    }

    // ── Connection handler ────────────────────────────────────────────────────

    /// Drive a single client connection until it closes or an I/O error occurs.
    async fn handle_connection(
        handler: Arc<RequestHandler>,
        stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
        peer_addr: String,
        event_rx: Receiver<FileEvent>,
    ) {
        let mut framed = Framed::new(stream, AlloyCodec::new());

        // Spawn a task that polls file-system events and sends them to this client.
        // We use a separate channel to push outgoing notifications into the write half.
        let (notif_tx, mut notif_rx) = tokio::sync::mpsc::channel::<RpcMessage>(256);

        let notif_task = {
            let peer = peer_addr.clone();
            tokio::spawn(async move {
                loop {
                    // Poll the crossbeam receiver without blocking the runtime.
                    let event = tokio::task::spawn_blocking({
                        let rx = event_rx.clone();
                        move || rx.recv()
                    })
                    .await;

                    match event {
                        Ok(Ok(file_event)) => {
                            let notif = Notification::FileChanged(file_event);
                            let params = match serde_json::to_value(&notif) {
                                Ok(v) => v,
                                Err(e) => {
                                    warn!("proxy: serialize FileChanged error: {e}");
                                    continue;
                                }
                            };
                            let msg = RpcMessage {
                                jsonrpc: RpcMessage::VERSION.to_owned(),
                                id: None,
                                method: Some("file_changed".to_string()),
                                params: Some(params),
                                result: None,
                                error: None,
                            };
                            if notif_tx.send(msg).await.is_err() {
                                debug!("proxy: notification channel closed for {peer}");
                                break;
                            }
                        }
                        Ok(Err(_)) => {
                            // Channel closed — no more events.
                            break;
                        }
                        Err(e) => {
                            warn!("proxy: spawn_blocking error polling events: {e}");
                            break;
                        }
                    }
                }
            })
        };

        loop {
            tokio::select! {
                // Outgoing notification.
                Some(msg) = notif_rx.recv() => {
                    if let Err(e) = framed.send(msg).await {
                        warn!("proxy: send notification to {peer_addr}: {e}");
                        break;
                    }
                }

                // Incoming request.
                incoming = framed.next() => {
                    match incoming {
                        None => {
                            info!("proxy: connection closed by {peer_addr}");
                            break;
                        }
                        Some(Err(e)) => {
                            warn!("proxy: framing error from {peer_addr}: {e}");
                            break;
                        }
                        Some(Ok(msg)) => {
                            if msg.is_request() {
                                // Extract id and method (both guaranteed present).
                                let id = msg.id.clone().unwrap();
                                let method = msg.method.clone().unwrap();
                                let params = msg.params.clone();

                                let response_msg = Self::dispatch(
                                    Arc::clone(&handler),
                                    id,
                                    &method,
                                    params,
                                )
                                .await;

                                if let Err(e) = framed.send(response_msg).await {
                                    warn!("proxy: send response to {peer_addr}: {e}");
                                    break;
                                }
                            } else if msg.is_notification() {
                                // Notifications from client — handle but no response.
                                let method = msg.method.clone().unwrap_or_default();
                                debug!("proxy: received notification '{method}' from {peer_addr}");
                            }
                            // Responses from client (shouldn't happen in server role) → ignore.
                        }
                    }
                }
            }
        }

        notif_task.abort();
    }

    // ── Dispatch ──────────────────────────────────────────────────────────────

    /// Dispatch a single request to the handler and build the response message.
    async fn dispatch(
        handler: Arc<RequestHandler>,
        id: RpcId,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> RpcMessage {
        let params = params.unwrap_or(serde_json::Value::Null);

        match method {
            "read_file" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                match handler.handle_read_file(&path).await {
                    Ok(content) => {
                        let resp = Response::ReadFile { content };
                        ok_response(id, resp)
                    }
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "write_file" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                // Content is base64-encoded or a JSON array of bytes.
                let content: Vec<u8> = match params.get("content") {
                    Some(serde_json::Value::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| v.as_u64().map(|b| b as u8))
                        .collect(),
                    Some(serde_json::Value::String(s)) => {
                        // Try to decode as base64; fall back to raw UTF-8 bytes.
                        if let Ok(decoded) = base64_decode(s) {
                            decoded
                        } else {
                            s.as_bytes().to_vec()
                        }
                    }
                    _ => vec![],
                };
                match handler.handle_write_file(&path, content).await {
                    Ok(()) => ok_response(id, Response::WriteFile),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "list_dir" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                match handler.handle_list_dir(&path).await {
                    Ok(entries) => ok_response(id, Response::ListDir { entries }),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "stat_file" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                match handler.handle_stat_file(&path).await {
                    Ok(stat) => ok_response(id, Response::StatFile { stat }),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "delete_file" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                match handler.handle_delete_file(&path).await {
                    Ok(()) => ok_response(id, Response::DeleteFile),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "create_dir" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                let recursive = params
                    .get("recursive")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                match handler.handle_create_dir(&path, recursive).await {
                    Ok(()) => ok_response(id, Response::CreateDir),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "watch_path" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                match handler.handle_watch_path(&path) {
                    Ok(()) => ok_response(id, Response::WatchPath),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            "unwatch_path" => {
                let path = match params.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return error_response(id, -32602, "missing 'path' param"),
                };
                match handler.handle_unwatch_path(&path) {
                    Ok(()) => ok_response(id, Response::UnwatchPath),
                    Err(e) => error_response(id, -32000, &e.to_string()),
                }
            }

            _ => error_response(id, -32601, &format!("method not found: {method}")),
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn ok_response(id: RpcId, resp: Response) -> RpcMessage {
    let result = serde_json::to_value(&resp).unwrap_or(serde_json::Value::Null);
    RpcMessage {
        jsonrpc: RpcMessage::VERSION.to_owned(),
        id: Some(id),
        method: None,
        params: None,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: RpcId, code: i32, message: &str) -> RpcMessage {
    let err = alloy_rpc::error::RpcError::new(code, message);
    RpcMessage::new_err(id, err)
}

/// Minimal base64 decoder (standard alphabet, no padding required).
///
/// We inline this to avoid adding a new dependency for a single utility use.
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut rev = [0u8; 256];
    for (i, &b) in TABLE.iter().enumerate() {
        rev[b as usize] = i as u8;
    }

    let input = input.trim_end_matches('=');
    let n = input.len();
    let mut out = Vec::with_capacity(n * 3 / 4 + 1);
    let bytes = input.as_bytes();

    let mut i = 0;
    while i + 3 < n {
        let (a, b, c, d) = (
            rev[bytes[i] as usize],
            rev[bytes[i + 1] as usize],
            rev[bytes[i + 2] as usize],
            rev[bytes[i + 3] as usize],
        );
        out.push((a << 2) | (b >> 4));
        out.push((b << 4) | (c >> 2));
        out.push((c << 6) | d);
        i += 4;
    }
    match n - i {
        2 => {
            let (a, b) = (rev[bytes[i] as usize], rev[bytes[i + 1] as usize]);
            out.push((a << 2) | (b >> 4));
        }
        3 => {
            let (a, b, c) = (
                rev[bytes[i] as usize],
                rev[bytes[i + 1] as usize],
                rev[bytes[i + 2] as usize],
            );
            out.push((a << 2) | (b >> 4));
            out.push((b << 4) | (c >> 2));
        }
        _ => {}
    }
    Ok(out)
}
