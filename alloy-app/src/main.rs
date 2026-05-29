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

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse_args();

    if args.version {
        println!("alloy {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    logging::init(&args.log_level);

    // Spin the entire backend (tokio + all subsystems) on a dedicated OS thread
    // so the main thread stays free for the Floem GPU event loop, which requires it.
    std::thread::Builder::new()
        .name("alloy-backend".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("alloy-worker")
                .build()
                .expect("failed to build tokio runtime");

            if let Err(e) = runtime.block_on(async {
                let application = app::App::new(args).await?;
                application.run().await
            }) {
                tracing::error!(error = %e, "backend exited with error");
            }
        })?;

    // Main thread: open the Floem window (must be on the main thread).
    floem::Application::new()
        .window(
            |_| alloy_ui::shell::editor_shell(),
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
