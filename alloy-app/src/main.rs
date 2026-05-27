//! `alloy` binary entry point.

mod app;
mod cli;
mod logging;
mod signal;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse_args();

    // Handle --version early so logging is not initialised unnecessarily.
    if args.version {
        println!("alloy {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    logging::init(&args.log_level);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("alloy-worker")
        .build()?;

    runtime.block_on(async {
        let application = app::App::new(args).await?;
        application.run().await
    })
}
