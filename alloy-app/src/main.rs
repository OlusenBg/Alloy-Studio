//! `alloy` binary entry point.

// Hide the console window on Windows release builds.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod cli;
mod logging;
mod signal;

use std::sync::Arc;

use alloy_core::config::AlloyConfig;
use alloy_core::workspace::Workspace;
use alloy_telemetry::stream::TelemetryStream;
use parking_lot::RwLock;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse_args();

    if args.version {
        println!("alloy {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    logging::init(&args.log_level);

    // ── Build shared state ────────────────────────────────────────────────────
    // These are created synchronously before the runtime, so both the backend
    // thread and the Floem UI can share them via Arc.

    // Resolve workspace root (mirrors what App::new() does).
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
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    // Config
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(AlloyConfig::default_config_path);
    let config: Arc<RwLock<AlloyConfig>> = Arc::new(RwLock::new(
        AlloyConfig::load(&config_path).unwrap_or_else(|_| AlloyConfig::default()),
    ));

    // Workspace
    let workspace = Arc::new(Workspace::new(workspace_root.clone()));
    if let Err(e) = workspace.refresh_file_tree() {
        tracing::warn!(error = %e, "failed to build initial file tree");
    }

    // Telemetry stream (the server will publish to this once running).
    let telemetry = Arc::new(TelemetryStream::new());

    // FTC project detection (sync, walk filesystem).
    let ftc_project = alloy_gradle::project::FtcProject::detect(&workspace_root).ok();

    // Git repository discovery (sync, just opens the .git dir).
    let git_repo = alloy_git::repo::Repository::discover(&workspace_root)
        .ok()
        .map(Arc::new);

    // ── Tokio runtime ─────────────────────────────────────────────────────────
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("alloy-worker")
        .build()
        .expect("failed to build tokio runtime");

    // Grab the handle *before* moving the runtime into the backend thread.
    let tokio_handle = runtime.handle().clone();

    // ── Build the AppBridge ───────────────────────────────────────────────────
    let bridge = Arc::new(alloy_ui::bridge::AppBridge {
        tokio: tokio_handle,
        workspace_root: workspace_root.clone(),
        workspace: Arc::clone(&workspace),
        config: Arc::clone(&config),
        telemetry: Arc::clone(&telemetry),
        ftc_project,
        git_repo,
    });

    // ── Backend thread ────────────────────────────────────────────────────────
    // The runtime (and all subsystems) lives on this thread so the main thread
    // stays free for the Floem GPU event loop.
    let args_clone = args.clone();
    let telemetry_clone = Arc::clone(&telemetry);
    std::thread::Builder::new()
        .name("alloy-backend".into())
        .spawn(move || {
            if let Err(e) = runtime.block_on(async {
                let application =
                    app::App::new_with_shared(args_clone, Arc::clone(&telemetry_clone)).await?;
                application.run().await
            }) {
                tracing::error!(error = %e, "backend exited with error");
            }
        })?;

    // ── Floem UI ──────────────────────────────────────────────────────────────
    // Must run on the main thread.
    floem::Application::new()
        .window(
            move |_| alloy_ui::shell::editor_shell(Some(Arc::clone(&bridge))),
            Some(
                floem::window::WindowConfig::default()
                    .title("Alloy Studio")
                    .size(floem::kurbo::Size::new(1440.0, 900.0))
                    .show_titlebar(false),
            ),
        )
        .run();

    Ok(())
}
