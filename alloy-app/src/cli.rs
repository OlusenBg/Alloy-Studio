//! Command-line interface definition for the `alloy` binary.

use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for Alloy Studio.
#[derive(Parser, Debug, Clone)]
#[command(name = "alloy", about = "Alloy Studio — FTC robotics code editor", version)]
pub struct Cli {
    /// Open a project directory or file
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Configuration file path
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Log level: error, warn, info, debug, trace
    #[arg(long, default_value = "info", env = "ALLOY_LOG")]
    pub log_level: String,

    /// Telemetry server port
    #[arg(long, default_value = "5800", env = "ALLOY_TELEMETRY_PORT")]
    pub telemetry_port: u16,

    /// IPC server TCP port (for UI connection)
    #[arg(long, default_value = "7700", env = "ALLOY_IPC_PORT")]
    pub ipc_port: u16,

    /// IPC socket path (Unix only, overrides --ipc-port)
    #[arg(long, value_name = "PATH")]
    pub ipc_socket: Option<PathBuf>,

    /// Print version and exit
    #[arg(long)]
    pub version: bool,
}

impl Cli {
    /// Parse arguments from `std::env::args_os`.
    pub fn parse_args() -> Self {
        <Self as clap::Parser>::parse()
    }
}
