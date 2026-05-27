//! OS signal handling for graceful shutdown.

use tokio::sync::broadcast;

/// A lightweight wrapper around a broadcast channel used to signal shutdown.
pub struct ShutdownSignal(broadcast::Sender<()>);

impl ShutdownSignal {
    /// Create a new shutdown signal.
    pub fn new() -> Self {
        Self(broadcast::channel(1).0)
    }

    /// Subscribe to the shutdown signal.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.0.subscribe()
    }

    /// Block the current async task until a shutdown signal is received from the OS.
    ///
    /// On Unix: waits for SIGTERM or Ctrl-C (SIGINT).
    /// On other platforms: waits for Ctrl-C only.
    pub async fn wait_for_shutdown() {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("SIGTERM received");
                }
                result = tokio::signal::ctrl_c() => {
                    if let Err(e) = result {
                        tracing::warn!("ctrl_c error: {e}");
                    } else {
                        tracing::info!("Ctrl-C received");
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::warn!("ctrl_c error: {e}");
            } else {
                tracing::info!("Ctrl-C received");
            }
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}
