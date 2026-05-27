//! Error types for the alloy-git crate.

/// All errors that can occur in the Git subsystem.
#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("not a git repository: {path}")]
    NotARepo { path: String },

    #[error("branch not found: {0}")]
    BranchNotFound(String),

    #[error("conflict detected in: {path}")]
    ConflictDetected { path: String },

    #[error("no commits yet")]
    NoCommits,

    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}
