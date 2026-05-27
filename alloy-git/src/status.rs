//! Git working-tree status.

use serde::{Deserialize, Serialize};

use crate::repo::Repository;

// ── StatusKind ────────────────────────────────────────────────────────────────

/// High-level classification of a file's status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    Untracked,
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Conflicted,
    Ignored,
}

// ── FileStatus ────────────────────────────────────────────────────────────────

/// Status of a single file in the working tree / index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    /// Current path (relative to repo root).
    pub path: String,
    /// Previous path, set for renames/copies.
    pub old_path: Option<String>,
    pub kind: StatusKind,
    /// `true` when the change is in the index (staged).
    pub staged: bool,
    /// Lines added (best-effort; 0 for untracked/deleted).
    pub additions: u32,
    /// Lines removed (best-effort; 0 for untracked/added).
    pub deletions: u32,
}

// ── GitStatus ─────────────────────────────────────────────────────────────────

/// Collection of all file statuses for the repository.
pub struct GitStatus(pub Vec<FileStatus>);

impl GitStatus {
    /// Collect the full working-tree + index status.
    pub async fn collect(repo: &Repository) -> anyhow::Result<Self> {
        let files = repo
            .with_repo(|r| {
                let mut opts = git2::StatusOptions::new();
                opts.include_untracked(true)
                    .include_ignored(false)
                    .recurse_untracked_dirs(true)
                    .renames_head_to_index(true)
                    .renames_index_to_workdir(true);

                let statuses = r.statuses(Some(&mut opts))?;
                let mut files = Vec::new();

                for entry in statuses.iter() {
                    let bits = entry.status();

                    // Skip truly unchanged entries.
                    if bits == git2::Status::CURRENT {
                        continue;
                    }

                    let path = entry
                        .path()
                        .unwrap_or("")
                        .to_string();

                    let old_path = entry
                        .head_to_index()
                        .and_then(|d| d.old_file().path())
                        .map(|p| p.to_string_lossy().into_owned())
                        .or_else(|| {
                            entry
                                .index_to_workdir()
                                .and_then(|d| d.old_file().path())
                                .map(|p| p.to_string_lossy().into_owned())
                        });

                    // Determine whether the change is staged.
                    let (kind, staged) = classify_status(bits);

                    // Compute additions/deletions from the relevant diff delta.
                    let (additions, deletions) =
                        diff_stats_for_entry(&entry, r).unwrap_or((0, 0));

                    files.push(FileStatus {
                        path,
                        old_path,
                        kind,
                        staged,
                        additions,
                        deletions,
                    });
                }

                Ok(files)
            })
            .await?;

        Ok(GitStatus(files))
    }

    /// Iterator over staged files.
    pub fn staged(&self) -> impl Iterator<Item = &FileStatus> {
        self.0.iter().filter(|f| f.staged)
    }

    /// Iterator over unstaged (working-tree) files.
    pub fn unstaged(&self) -> impl Iterator<Item = &FileStatus> {
        self.0
            .iter()
            .filter(|f| !f.staged && f.kind != StatusKind::Untracked)
    }

    /// Iterator over untracked files.
    pub fn untracked(&self) -> impl Iterator<Item = &FileStatus> {
        self.0
            .iter()
            .filter(|f| f.kind == StatusKind::Untracked)
    }

    /// Returns `true` if any file has a conflict marker.
    pub fn has_conflicts(&self) -> bool {
        self.0.iter().any(|f| f.kind == StatusKind::Conflicted)
    }

    /// All file statuses.
    pub fn files(&self) -> &[FileStatus] {
        &self.0
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Map git2 status bits to a `(StatusKind, staged)` pair.
///
/// We take the "most important" status bit.  Conflict > index > workdir.
fn classify_status(bits: git2::Status) -> (StatusKind, bool) {
    // Conflicts (both index and workdir flags set simultaneously).
    if bits.contains(git2::Status::CONFLICTED) {
        return (StatusKind::Conflicted, false);
    }

    // Index (staged) flags.
    if bits.contains(git2::Status::INDEX_NEW) {
        return (StatusKind::Added, true);
    }
    if bits.contains(git2::Status::INDEX_MODIFIED) {
        return (StatusKind::Modified, true);
    }
    if bits.contains(git2::Status::INDEX_DELETED) {
        return (StatusKind::Deleted, true);
    }
    if bits.contains(git2::Status::INDEX_RENAMED) {
        return (StatusKind::Renamed, true);
    }
    if bits.contains(git2::Status::INDEX_TYPECHANGE) {
        return (StatusKind::Modified, true);
    }

    // Workdir (unstaged) flags.
    if bits.contains(git2::Status::WT_NEW) {
        return (StatusKind::Untracked, false);
    }
    if bits.contains(git2::Status::WT_MODIFIED) {
        return (StatusKind::Modified, false);
    }
    if bits.contains(git2::Status::WT_DELETED) {
        return (StatusKind::Deleted, false);
    }
    if bits.contains(git2::Status::WT_RENAMED) {
        return (StatusKind::Renamed, false);
    }
    if bits.contains(git2::Status::WT_TYPECHANGE) {
        return (StatusKind::Modified, false);
    }
    if bits.contains(git2::Status::IGNORED) {
        return (StatusKind::Ignored, false);
    }

    (StatusKind::Modified, false)
}

/// Compute line additions/deletions for a single status entry.
fn diff_stats_for_entry(
    entry: &git2::StatusEntry<'_>,
    repo: &git2::Repository,
) -> anyhow::Result<(u32, u32)> {
    // Try staged diff first (index vs HEAD).
    if let Some(delta) = entry.head_to_index() {
        if let (Some(stats), false) = (patch_stats(repo, &delta)?, delta.new_file().is_binary()) {
            return Ok(stats);
        }
    }
    // Fall back to unstaged diff (workdir vs index).
    if let Some(delta) = entry.index_to_workdir() {
        if let (Some(stats), false) = (patch_stats(repo, &delta)?, delta.new_file().is_binary()) {
            return Ok(stats);
        }
    }
    Ok((0, 0))
}

/// Compute (additions, deletions) from a `DiffDelta` via a temporary `Patch`.
fn patch_stats(
    repo: &git2::Repository,
    delta: &git2::DiffDelta<'_>,
) -> anyhow::Result<Option<(u32, u32)>> {
    // git2 provides `Patch::from_diff` but we need the parent diff object.
    // Since we only have a delta here, use blob-to-blob diffing.
    let old_blob = if delta.old_file().id().is_zero() {
        None
    } else {
        repo.find_blob(delta.old_file().id()).ok()
    };

    let new_blob = if delta.new_file().id().is_zero() {
        None
    } else {
        repo.find_blob(delta.new_file().id()).ok()
    };

    let patch = git2::Patch::from_blobs(
        old_blob.as_ref(),
        None,
        new_blob.as_ref(),
        None,
        None,
    )?;

    let stats = patch.line_stats()?;
    Ok(Some((stats.1 as u32, stats.2 as u32)))
}
