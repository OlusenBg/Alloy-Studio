//! Shared value types used across the RPC protocol.

use serde::{Deserialize, Serialize};

/// A position in a text document, using zero-based line and character offsets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// A range in a text document expressed as start and end positions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Construct a range covering a single line from `start_char` to `end_char`.
    pub fn on_line(line: u32, start_char: u32, end_char: u32) -> Self {
        Self {
            start: Position::new(line, start_char),
            end: Position::new(line, end_char),
        }
    }
}

/// A textual edit operation: replace `range` with `new_text`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: Range, new_text: impl Into<String>) -> Self {
        Self {
            range,
            new_text: new_text.into(),
        }
    }
}

/// Severity of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

impl From<DiagnosticSeverity> for u8 {
    fn from(s: DiagnosticSeverity) -> u8 {
        s as u8
    }
}

impl TryFrom<u8> for DiagnosticSeverity {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match v {
            1 => Ok(DiagnosticSeverity::Error),
            2 => Ok(DiagnosticSeverity::Warning),
            3 => Ok(DiagnosticSeverity::Information),
            4 => Ok(DiagnosticSeverity::Hint),
            _ => Err(format!("unknown DiagnosticSeverity value: {v}")),
        }
    }
}

/// A compiler or linter diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(range: Range, message: impl Into<String>) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Error,
            code: None,
            source: None,
            message: message.into(),
        }
    }

    pub fn warning(range: Range, message: impl Into<String>) -> Self {
        Self {
            range,
            severity: DiagnosticSeverity::Warning,
            code: None,
            source: None,
            message: message.into(),
        }
    }
}

/// A location inside a document (URI + range).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn new(uri: impl Into<String>, range: Range) -> Self {
        Self {
            uri: uri.into(),
            range,
        }
    }
}

/// A completion item returned by the language server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
}

impl CompletionItem {
    pub fn simple(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: None,
            detail: None,
            documentation: None,
            insert_text: None,
        }
    }
}

/// Symbol information for workspace-wide symbol search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolInformation {
    pub name: String,
    /// LSP SymbolKind value (1 = File, 2 = Module, 5 = Class, 6 = Method, …)
    pub kind: u32,
    pub location: Location,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

/// The kind of a file system change event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEventKind {
    Created,
    Changed,
    Deleted,
}

/// A file system change event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEvent {
    pub uri: String,
    pub kind: FileEventKind,
}

impl FileEvent {
    pub fn new(uri: impl Into<String>, kind: FileEventKind) -> Self {
        Self {
            uri: uri.into(),
            kind,
        }
    }
}

/// The category of a build error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildErrorKind {
    Compile,
    Dependency,
    Gradle,
    Sdk,
}

/// A structured build error, optionally tied to a file location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    pub message: String,
    pub kind: BuildErrorKind,
}

impl BuildError {
    pub fn new(message: impl Into<String>, kind: BuildErrorKind) -> Self {
        Self {
            file: None,
            line: None,
            column: None,
            message: message.into(),
            kind,
        }
    }

    pub fn compile(message: impl Into<String>) -> Self {
        Self::new(message, BuildErrorKind::Compile)
    }
}

/// Events emitted by the build subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuildEvent {
    OutputLine(String),
    ErrorDetected(BuildError),
    Finished {
        exit_code: i32,
        errors: Vec<BuildError>,
    },
}

/// A telemetry data point streamed from the robot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    KeyValue {
        key: String,
        value: f64,
    },
    EncoderTick {
        name: String,
        ticks: i64,
        velocity_tps: f64,
    },
    Battery {
        volts: f64,
    },
    Gyro {
        heading_deg: f64,
    },
    Imu {
        roll: f64,
        pitch: f64,
        yaw: f64,
    },
    Ping,
}

/// Log message severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// An entry in a directory listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_ms: Option<i64>,
}

/// Metadata for a single file system path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileStat {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_ms: Option<i64>,
}
