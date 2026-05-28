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

                    let path = entry.path().unwrap_or("").to_string();

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
                    let (additions, deletions) = diff_stats_for_entry(&entry, r).unwrap_or((0, 0));

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
        self.0.iter().filter(|f| f.kind == StatusKind::Untracked)
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
///
/// We take a best-effort approach using the pre-computed `additions`/`deletions`
/// stored on the `FileStatus` itself (populated from git2 status flags), or by
/// doing a blob diff when both sides have known OIDs.
fn diff_stats_for_entry(
    entry: &git2::StatusEntry<'_>,
    repo: &git2::Repository,
) -> anyhow::Result<(u32, u32)> {
    // Try staged diff first (index vs HEAD).
    if let Some(delta) = entry.head_to_index() {
        if !delta.new_file().is_binary() {
            if let Some(stats) = patch_stats_for_delta(repo, &delta)? {
                return Ok(stats);
            }
        }
    }
    // Fall back to unstaged diff (workdir vs index).
    if let Some(delta) = entry.index_to_workdir() {
        if !delta.new_file().is_binary() {
            if let Some(stats) = patch_stats_for_delta(repo, &delta)? {
                return Ok(stats);
            }
        }
    }
    Ok((0, 0))
}

/// Try to compute `(additions, deletions)` from a single `DiffDelta`.
///
/// Returns `None` when the OIDs needed for diffing are unavailable (e.g. for
/// untracked files or newly added working-tree files whose index OID is zero).
fn patch_stats_for_delta(
    repo: &git2::Repository,
    delta: &git2::DiffDelta<'_>,
) -> anyhow::Result<Option<(u32, u32)>> {
    let old_id = delta.old_file().id();
    let new_id = delta.new_file().id();

    match (old_id.is_zero(), new_id.is_zero()) {
        (false, false) => {
            // Both sides are known blobs — do a proper diff.
            let old_blob = repo.find_blob(old_id)?;
            let new_blob = repo.find_blob(new_id)?;
            let patch = git2::Patch::from_blobs(&old_blob, None, &new_blob, None, None)?;
            let stats = patch.line_stats()?;
            Ok(Some((stats.1 as u32, stats.2 as u32)))
        }
        (true, false) => {
            // File newly added — count all lines as additions.
            let new_blob = repo.find_blob(new_id)?;
            let adds = count_lines(new_blob.content());
            Ok(Some((adds, 0)))
        }
        (false, true) => {
            // File deleted — count all lines as deletions.
            let old_blob = repo.find_blob(old_id)?;
            let dels = count_lines(old_blob.content());
            Ok(Some((0, dels)))
        }
        (true, true) => Ok(None),
    }
}

/// Count newline-separated lines in a byte slice.
fn count_lines(bytes: &[u8]) -> u32 {
    if bytes.is_empty() {
        return 0;
    }
    // Count LF bytes; add 1 if the last byte is not LF.
    let lf_count = bytes.iter().filter(|&&b| b == b'\n').count() as u32;
    if bytes.last() == Some(&b'\n') {
        lf_count
    } else {
        lf_count + 1
    }
}
