//! `alloy-gradle` — Gradle + FTC build tooling: runner, error parser, repair engine,
//! OpMode scanner, and hardware config.

pub mod error;
pub mod hardware;
pub mod opmode;
pub mod parser;
pub mod project;
pub mod repair;
pub mod runner;

// Top-level re-exports
pub use error::GradleError;
pub use hardware::{DeviceDeclaration, HardwareConfig, PortAssignment};
pub use opmode::{OpModeInfo, OpModeKind, OpModeScanner};
pub use parser::BuildOutputParser;
pub use project::FtcProject;
pub use repair::{RepairEngine, RepairPatch, RepairSuggestion};
pub use runner::{GradleRunner, GradleTask};
