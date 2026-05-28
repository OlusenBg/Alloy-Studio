//! Tracing-subscriber initialisation.

/// Initialise a global tracing subscriber.
///
/// The effective level is determined by (in priority order):
/// 1. The `RUST_LOG` environment variable (via [`tracing_subscriber::EnvFilter`]).
/// 2. The `log_level` string supplied by the CLI.
pub fn init(log_level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .init();
}
