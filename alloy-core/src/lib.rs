//! `alloy-core` — Editor primitives: buffer, syntax, config, workspace, search.

pub mod buffer;
pub mod config;
pub mod document;
pub mod error;
pub mod search;
pub mod selection;
pub mod syntax;
pub mod workspace;

// --- Top-level re-exports ---------------------------------------------------

pub use buffer::{Buffer, EditOp};
pub use config::{AlloyConfig, ConfigWatcher, EditorConfig, FtcConfig, GitConfig, LineEnding};
pub use document::Document;
pub use error::{CoreError, Result};
pub use search::{BufferSearcher, SearchMatch, WorkspaceSearcher};
pub use selection::{Cursor, Selection, SelectionSet};
pub use syntax::{HighlightRange, SyntaxLayer, SyntaxRegistry};
pub use workspace::{FileEntry, Workspace};
