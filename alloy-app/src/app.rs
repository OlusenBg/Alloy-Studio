//! Main application struct: wires all subsystems together and drives the run loop.

use std::path::PathBuf;
use std::sync::Arc;

use alloy_core::config::AlloyConfig;
use alloy_core::workspace::Workspace;
use alloy_lsp::manager::LspManager;
use alloy_proxy::server::ProxyServer;
use alloy_telemetry::server::TelemetryServer;
use alloy_telemetry::stream::TelemetryStream;
use parking_lot::RwLock;

use crate::cli::Cli;

/// Top-level application: owns and coordinates all subsystems.
pub struct App {
    #[allow(dead_code)]
    config: Arc<RwLock<AlloyConfig>>,
    #[allow(dead_code)]
    workspace: Arc<Workspace>,
    lsp_manager: Arc<LspManager>,
    telemetry_server: TelemetryServer,
    #[allow(dead_code)]
    telemetry_stream: Arc<TelemetryStream>,
    proxy_server: Arc<ProxyServer>,
    ipc_port: u16,
    ipc_socket: Option<PathBuf>,
    telemetry_port: u16,
}

impl App {
    /// Initialise all subsystems, reusing a pre-built `TelemetryStream`.
    ///
    /// This variant is used when the UI bridge holds a reference to the same
    /// telemetry stream, so robot packets are visible to both the backend and the UI.
    pub async fn new_with_shared(
        args: Cli,
        telemetry_stream: Arc<TelemetryStream>,
    ) -> anyhow::Result<Self> {
        // 1. Load config.
        let config_path = args
            .config
            .clone()
            .unwrap_or_else(AlloyConfig::default_config_path);
        let config = Arc::new(RwLock::new(
            AlloyConfig::load(&config_path).unwrap_or_else(|_| AlloyConfig::default()),
        ));

        // 2. Detect workspace root.
        let workspace_root = args
            .path
            .as_ref()
            .map(|p| {
                if p.is_file() {
                    p.parent()
                        .map(|parent| parent.to_path_buf())
                        .unwrap_or_else(|| p.clone())
                } else {
                    p.clone()
                }
            })
            .map(|p| Workspace::detect_root(&p))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let workspace = Arc::new(Workspace::new(workspace_root.clone()));
        if let Err(e) = workspace.refresh_file_tree() {
            tracing::warn!(error = %e, "failed to build initial file tree");
        }

        // 3. LSP manager.
        let lsp_manager = Arc::new(LspManager::new(workspace_root.clone(), Arc::clone(&config)));

        // 4. Telemetry server (reuses the provided shared stream).
        let telemetry_server =
            TelemetryServer::bind_with_stream(args.telemetry_port, Arc::clone(&telemetry_stream))
                .await?;

        // 5. Proxy server.
        let proxy_server = Arc::new(ProxyServer::new()?);

        Ok(Self {
            config,
            workspace,
            lsp_manager,
            telemetry_server,
            telemetry_stream,
            proxy_server,
            ipc_port: args.ipc_port,
            ipc_socket: args.ipc_socket,
            telemetry_port: args.telemetry_port,
        })
    }

    /// Initialise all subsystems from the parsed CLI arguments.
    #[allow(dead_code)]
    pub async fn new(args: Cli) -> anyhow::Result<Self> {
        // 1. Load config (from args.config or default path).
        let config_path = args.config.unwrap_or_else(AlloyConfig::default_config_path);

        let config = Arc::new(RwLock::new(
            AlloyConfig::load(&config_path).unwrap_or_else(|_| AlloyConfig::default()),
        ));

        // 2. Detect workspace root.
        let workspace_root = args
            .path
            .map(|p| {
                if p.is_file() {
                    p.parent().map(|parent| parent.to_path_buf()).unwrap_or(p)
                } else {
                    p
                }
            })
            .map(|p| Workspace::detect_root(&p))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let workspace = Arc::new(Workspace::new(workspace_root.clone()));

        // Refresh the file tree; log but don't abort if the root doesn't exist.
        if let Err(e) = workspace.refresh_file_tree() {
            tracing::warn!(error = %e, "failed to build initial file tree");
        }

        // 3. LSP manager.
        let lsp_manager = Arc::new(LspManager::new(workspace_root.clone(), Arc::clone(&config)));

        // 4. Telemetry server.
        let (telemetry_server, telemetry_stream) =
            TelemetryServer::bind(args.telemetry_port).await?;

        // 5. Proxy server.
        let proxy_server = Arc::new(ProxyServer::new()?);

        Ok(Self {
            config,
            workspace,
            lsp_manager,
            telemetry_server,
            telemetry_stream,
            proxy_server,
            ipc_port: args.ipc_port,
            ipc_socket: args.ipc_socket,
            telemetry_port: args.telemetry_port,
        })
    }

    /// Start all subsystems and run until a shutdown signal is received.
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!(version = env!("CARGO_PKG_VERSION"), "Alloy Studio starting");

        // Destructure to avoid partial-move issues when moving into async tasks.
        let Self {
            lsp_manager,
            telemetry_server,
            proxy_server,
            ipc_port,
            ipc_socket,
            telemetry_port,
            ..
        } = self;

        // ── Proxy IPC task ────────────────────────────────────────────────────
        let proxy_task = {
            let proxy = Arc::clone(&proxy_server);
            tokio::spawn(async move {
                #[cfg(unix)]
                if let Some(socket_path) = ipc_socket {
                    if let Err(e) = proxy.listen_unix(&socket_path).await {
                        tracing::error!(error = %e, "Proxy Unix socket error");
                    }
                    return;
                }

                #[cfg(not(unix))]
                let _ = ipc_socket; // unused on non-Unix

                let addr: std::net::SocketAddr = format!("127.0.0.1:{ipc_port}").parse().unwrap();
                if let Err(e) = proxy.listen_tcp(addr).await {
                    tracing::error!(error = %e, "Proxy TCP server error");
                }
            })
        };

        // ── Telemetry task ────────────────────────────────────────────────────
        let telemetry_task = tokio::spawn(async move {
            if let Err(e) = telemetry_server.run().await {
                tracing::error!(error = %e, "Telemetry server error");
            }
        });

        tracing::info!(ipc_port, telemetry_port, "All subsystems started");

        // ── Wait for shutdown ─────────────────────────────────────────────────
        crate::signal::ShutdownSignal::wait_for_shutdown().await;

        tracing::info!("Shutting down…");

        lsp_manager.shutdown_all().await;
        proxy_task.abort();
        telemetry_task.abort();

        tracing::info!("Shutdown complete");
        Ok(())
    }
}
