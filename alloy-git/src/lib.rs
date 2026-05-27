//! `alloy-git` — Git integration: status, diff, blame, branch operations,
//! conflict resolution, and AI-assisted commit message generation.

pub mod ai_message;
pub mod blame;
pub mod branch;
pub mod commit;
pub mod conflict;
pub mod diff;
pub mod error;
pub mod repo;
pub mod status;

pub use ai_message::CommitMessageBuilder;
pub use blame::{BlameLine, GitBlame};
pub use branch::{Branch, BranchList};
pub use commit::CommitOptions;
pub use conflict::{ConflictFile, ConflictHunk, ConflictSide, ResolutionChoice};
pub use diff::{DiffHunk, DiffLine, DiffLineKind, FileDiff};
pub use error::GitError;
pub use repo::Repository;
pub use status::{FileStatus, GitStatus, StatusKind};
