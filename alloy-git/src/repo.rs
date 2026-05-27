//! Thread-safe wrapper around `git2::Repository`.
//!
//! Because `git2::Repository` is not `Send`, every operation that needs it
//! must be dispatched through `tokio::task::spawn_blocking`.  The wrapper
//! stores only the path so the repository can be re-opened inside each
//! blocking closure.

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::error::GitError;

// ── Repository ────────────────────────────────────────────────────────────────

/// A git repository reference.  Cheap to clone (stores only the workdir path).
#[derive(Debug, Clone)]
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// Open the repository rooted at exactly `path`.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        // Validate that git2 can open it.
        git2::Repository::open(path)
            .with_context(|| format!("opening git repo at {}", path.display()))?;

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Discover the repository by walking up the directory tree from `start`.
    pub fn discover(start: &Path) -> anyhow::Result<Self> {
        let repo = git2::Repository::discover(start)
            .with_context(|| format!("discovering git repo from {}", start.display()))?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::NotARepo {
                path: start.to_string_lossy().into_owned(),
            })?
            .to_path_buf();

        Ok(Self { path: workdir })
    }

    /// Return the workdir path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Run `f` inside `tokio::task::spawn_blocking`, receiving a freshly-opened
    /// `git2::Repository`.
    ///
    /// This is the **only** correct way to call git2 APIs from async code in
    /// alloy-git, because `git2::Repository` is `!Send`.
    pub async fn with_repo<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&git2::Repository) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let repo = git2::Repository::open(&path)
                .with_context(|| format!("opening git repo at {}", path.display()))?;
            f(&repo)
        })
        .await
        .context("spawn_blocking panicked")?
    }

    /// Synchronous helper: return the HEAD commit OID as a hex string.
    ///
    /// Intended for use inside `with_repo` closures where async is unavailable.
    pub fn head_commit_id_sync(repo: &git2::Repository) -> anyhow::Result<String> {
        let head = repo.head().context("reading HEAD")?;
        let oid = head
            .target()
            .ok_or_else(|| anyhow::anyhow!("HEAD is not a direct reference"))?;
        Ok(oid.to_string())
    }
}
