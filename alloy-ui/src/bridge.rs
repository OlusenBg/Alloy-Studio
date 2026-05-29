//! AppBridge — passes real backend state from alloy-app into the Floem UI.

use std::sync::Arc;

use parking_lot::RwLock;

/// Passed from alloy-app into editor_shell() — gives the UI access to real backend state.
pub struct AppBridge {
    /// Tokio runtime handle — spawn async tasks from UI event handlers.
    pub tokio: tokio::runtime::Handle,
    /// Workspace root path.
    pub workspace_root: std::path::PathBuf,
    /// Shared workspace (file tree, open documents).
    pub workspace: Arc<alloy_core::workspace::Workspace>,
    /// App config.
    pub config: Arc<RwLock<alloy_core::config::AlloyConfig>>,
    /// Live telemetry stream from robot.
    pub telemetry: Arc<alloy_telemetry::stream::TelemetryStream>,
    /// Detected FTC project (None if not an FTC project).
    pub ftc_project: Option<alloy_gradle::project::FtcProject>,
    /// Git repository (None if not a git repo).
    pub git_repo: Option<Arc<alloy_git::repo::Repository>>,
}
