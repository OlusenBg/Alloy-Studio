//! Typed response and notification enums.

use serde::{Deserialize, Serialize};

use crate::types::{
    CompletionItem, Diagnostic, DirEntry, FileStat, FileEvent, Location, LogLevel,
    SymbolInformation, TelemetryEvent, BuildEvent,
};

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// Every response the backend can send back to a client request.
///
/// Serialises as:
/// ```json
/// { "method": "read_file", "params": { "content": […] } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Response {
    // --- Generic outcomes ----------------------------------------------------

    /// Generic success response carrying no payload.
    Ok,

    /// A request was rejected with an error message.
    Error { message: String, code: i32 },

    // --- Document lifecycle --------------------------------------------------

    /// Confirmation that a document was opened.
    OpenDocument,

    /// Confirmation that a document was closed.
    CloseDocument,

    /// Confirmation that document changes were applied.
    ChangeDocument,

    /// Confirmation that the document was saved.
    SaveDocument,

    // --- Language features ---------------------------------------------------

    /// Completion items at the requested position.
    GetCompletion { items: Vec<CompletionItem> },

    /// Hover content (Markdown) at the requested position.
    GetHover { markdown: Option<String> },

    /// Definition locations for the symbol at the requested position.
    GetDefinition { locations: Vec<Location> },

    /// Reference locations for the symbol at the requested position.
    GetReferences { locations: Vec<Location> },

    /// Workspace symbols matching the query.
    GetWorkspaceSymbols { symbols: Vec<SymbolInformation> },

    /// Formatted document text edits.
    FormatDocument { edits: Vec<crate::types::TextEdit> },

    /// Current diagnostics for a document.
    GetDiagnostics { diagnostics: Vec<Diagnostic> },

    // --- Filesystem ----------------------------------------------------------

    /// Raw byte content of a file.
    ReadFile { content: Vec<u8> },

    /// Confirmation that a file was written.
    WriteFile,

    /// Directory listing.
    ListDir { entries: Vec<DirEntry> },

    /// File metadata.
    StatFile { stat: FileStat },

    /// Confirmation that a file or directory was deleted.
    DeleteFile,

    /// Confirmation that a directory was created.
    CreateDir,

    // --- Search --------------------------------------------------------------

    /// Matches found in a single buffer.
    SearchInBuffer { matches: Vec<serde_json::Value> },

    /// Matches found across the workspace.
    SearchWorkspace { matches: Vec<serde_json::Value> },

    // --- Build ---------------------------------------------------------------

    /// Build started; build events will arrive as `BuildOutput` notifications.
    StartBuild,

    /// Build was cancelled.
    CancelBuild,

    /// Repair suggestions for build failures.
    GetRepairSuggestions { suggestions: Vec<serde_json::Value> },

    /// Confirmation that a repair was applied.
    ApplyRepair,

    // --- Git -----------------------------------------------------------------

    /// Working-tree status entries.
    GitStatus { files: Vec<serde_json::Value> },

    /// Diff hunks.
    GitDiff { diff: Vec<serde_json::Value> },

    /// Line-level blame annotations.
    GitBlame { lines: Vec<serde_json::Value> },

    /// Branch list and the name of the currently checked-out branch.
    GitListBranches {
        branches: Vec<serde_json::Value>,
        current: Option<String>,
    },

    /// Confirmation that a branch checkout succeeded.
    GitCheckout,

    /// Confirmation that a commit was made.
    GitCommit,

    /// Confirmation that paths were staged.
    GitStage,

    /// Confirmation that paths were unstaged.
    GitUnstage,

    /// AI-generated commit message.
    GenerateCommitMessage { message: String },

    // --- FTC-specific --------------------------------------------------------

    /// OpMode class descriptors found in the project.
    ScanOpModes { opmodes: Vec<serde_json::Value> },

    /// Parsed hardware configuration assignments.
    ParseHardwareConfig { assignments: Vec<serde_json::Value> },

    /// Generated Java hardware field declarations.
    GenerateHardwareDeclarations { declarations: String },

    // --- Telemetry -----------------------------------------------------------

    /// Confirmation that telemetry subscription was registered.
    SubscribeTelemetry,

    /// Confirmation that telemetry subscription was removed.
    UnsubscribeTelemetry,

    /// Historical telemetry records for a key.
    GetTelemetryHistory { records: Vec<serde_json::Value> },

    // --- File watch ----------------------------------------------------------

    /// Confirmation that a path is now being watched.
    WatchPath,

    /// Confirmation that a path is no longer being watched.
    UnwatchPath,

    // --- Workspace -----------------------------------------------------------

    /// Confirmation that the workspace was opened.
    OpenWorkspace,

    /// Confirmation that the workspace was closed.
    CloseWorkspace,

    /// Recursive file tree rooted at the requested path.
    GetFileTree { entries: Vec<serde_json::Value> },
}

impl Response {
    /// Returns `true` if this is an error response.
    pub fn is_error(&self) -> bool {
        matches!(self, Response::Error { .. })
    }

    /// Convenience constructor for a generic error response.
    pub fn error(message: impl Into<String>, code: i32) -> Self {
        Response::Error {
            message: message.into(),
            code,
        }
    }
}

// ---------------------------------------------------------------------------
// Notification
// ---------------------------------------------------------------------------

/// Server-to-client push events that are not tied to a specific request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Notification {
    /// A fresh set of diagnostics for a document.
    PublishDiagnostics {
        uri: String,
        version: Option<i32>,
        diagnostics: Vec<Diagnostic>,
    },

    /// A file system change was detected.
    FileChanged(FileEvent),

    /// An event from the currently running build.
    BuildOutput(BuildEvent),

    /// A real-time telemetry data point from the robot.
    TelemetryUpdate(TelemetryEvent),

    /// A window-level log message from the backend.
    WindowLog { level: LogLevel, message: String },

    /// The workspace has been fully indexed and is ready.
    WorkspaceReady { root: String },
}
