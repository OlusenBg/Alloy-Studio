//! Typed request enum — one variant per RPC method.

use serde::{Deserialize, Serialize};

use crate::types::{Position, TextEdit};

/// Every request the frontend (or any client) can send to the backend.
///
/// The enum is serde-tagged so that it serialises as:
/// ```json
/// { "method": "open_document", "params": { … } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Request {
    // --- Document lifecycle --------------------------------------------------

    /// Notify that a document has been opened in the editor.
    OpenDocument {
        uri: String,
        language_id: String,
        version: i32,
        text: String,
    },

    /// Notify that a document has been closed.
    CloseDocument { uri: String },

    /// Apply incremental or full-text changes to an open document.
    ChangeDocument {
        uri: String,
        version: i32,
        edits: Vec<TextEdit>,
        full_text: Option<String>,
    },

    /// Notify that the user has saved a document.
    SaveDocument { uri: String },

    // --- Language features ---------------------------------------------------

    /// Request completion items at a given position.
    GetCompletion {
        uri: String,
        position: Position,
        trigger_character: Option<String>,
    },

    /// Request hover information at a given position.
    GetHover { uri: String, position: Position },

    /// Request the definition location for the symbol at `position`.
    GetDefinition { uri: String, position: Position },

    /// Request all references to the symbol at `position`.
    GetReferences {
        uri: String,
        position: Position,
        include_declaration: bool,
    },

    /// Search for workspace-wide symbols matching `query`.
    GetWorkspaceSymbols { query: String },

    /// Request a formatted version of the document.
    FormatDocument { uri: String },

    /// Request the current set of diagnostics for a document.
    GetDiagnostics { uri: String },

    // --- Filesystem (proxy ops) ----------------------------------------------

    /// Read the byte content of a file.
    ReadFile { path: String },

    /// Write byte content to a file, creating it if necessary.
    WriteFile { path: String, content: Vec<u8> },

    /// List the entries in a directory.
    ListDir { path: String },

    /// Retrieve metadata for a path.
    StatFile { path: String },

    /// Delete a file or empty directory.
    DeleteFile { path: String },

    /// Create a directory, optionally creating parent directories.
    CreateDir { path: String, recursive: bool },

    // --- Search --------------------------------------------------------------

    /// Search for a pattern in a single open buffer.
    SearchInBuffer {
        uri: String,
        pattern: String,
        is_regex: bool,
    },

    /// Search across all files in a workspace.
    SearchWorkspace {
        root: String,
        pattern: String,
        is_regex: bool,
        file_glob: Option<String>,
    },

    // --- Build ---------------------------------------------------------------

    /// Start a Gradle build for the given project.
    StartBuild {
        project_root: String,
        tasks: Vec<String>,
    },

    /// Cancel any running build.
    CancelBuild,

    /// Ask the backend for repair suggestions for build failures.
    GetRepairSuggestions { project_root: String },

    /// Apply a specific repair suggestion by index.
    ApplyRepair {
        project_root: String,
        suggestion_index: usize,
    },

    // --- Git -----------------------------------------------------------------

    /// Get the working-tree status of a repository.
    GitStatus { repo_root: String },

    /// Get a diff for the repository or a specific path.
    GitDiff {
        repo_root: String,
        path: Option<String>,
    },

    /// Get line-by-line blame annotations for a file.
    GitBlame { repo_root: String, path: String },

    /// List all branches in the repository.
    GitListBranches { repo_root: String },

    /// Check out a branch.
    GitCheckout { repo_root: String, branch: String },

    /// Commit staged changes with the given message.
    GitCommit {
        repo_root: String,
        message: String,
        paths: Vec<String>,
    },

    /// Stage specific paths.
    GitStage {
        repo_root: String,
        paths: Vec<String>,
    },

    /// Unstage specific paths.
    GitUnstage {
        repo_root: String,
        paths: Vec<String>,
    },

    /// Ask the AI to generate a commit message for staged changes.
    GenerateCommitMessage { repo_root: String },

    // --- FTC-specific --------------------------------------------------------

    /// Scan all OpMode classes in the project.
    ScanOpModes { project_root: String },

    /// Parse a hardware map configuration XML file.
    ParseHardwareConfig { file_path: String },

    /// Generate Java hardware field declarations from parsed assignments.
    GenerateHardwareDeclarations {
        assignments: Vec<serde_json::Value>,
    },

    // --- Telemetry -----------------------------------------------------------

    /// Subscribe to real-time telemetry updates.
    SubscribeTelemetry,

    /// Unsubscribe from telemetry updates.
    UnsubscribeTelemetry,

    /// Retrieve the historical telemetry records for a key.
    GetTelemetryHistory { key: String },

    // --- File watch ----------------------------------------------------------

    /// Watch a path for file system changes.
    WatchPath { path: String },

    /// Stop watching a previously watched path.
    UnwatchPath { path: String },

    // --- Workspace -----------------------------------------------------------

    /// Open a workspace rooted at `root`.
    OpenWorkspace { root: String },

    /// Close the currently open workspace.
    CloseWorkspace,

    /// Retrieve a recursive file tree rooted at `root`.
    GetFileTree { root: String },
}
