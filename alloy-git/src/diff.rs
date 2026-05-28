//! Git diff types and computation.

use serde::{Deserialize, Serialize};

use crate::repo::Repository;

// ── DiffLineKind ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
    NoNewline,
}

// ── DiffLine ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// Line number in the old file (1-based), if applicable.
    pub old_lineno: Option<u32>,
    /// Line number in the new file (1-based), if applicable.
    pub new_lineno: Option<u32>,
    /// Raw line content (including newline character).
    pub content: String,
}

// ── DiffHunk ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Unified-diff header string, e.g. `"@@ -1,3 +1,4 @@"`.
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

// ── FileDiff ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// Path in the new tree (relative to repo root).
    pub path: String,
    /// Old path, set for renames.
    pub old_path: Option<String>,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
    pub additions: u32,
    pub deletions: u32,
}

impl FileDiff {
    /// Diff workdir vs index (unstaged changes).
    pub async fn workdir_vs_index(
        repo: &Repository,
        path: Option<&str>,
    ) -> anyhow::Result<Vec<Self>> {
        let path = path.map(|s| s.to_string());
        repo.with_repo(move |r| {
            let mut diff_opts = git2::DiffOptions::new();
            if let Some(ref p) = path {
                diff_opts.pathspec(p);
            }
            let diff = r.diff_index_to_workdir(None, Some(&mut diff_opts))?;
            parse_diff(&diff)
        })
        .await
    }

    /// Diff index vs HEAD (staged changes).
    pub async fn index_vs_head(repo: &Repository, path: Option<&str>) -> anyhow::Result<Vec<Self>> {
        let path = path.map(|s| s.to_string());
        repo.with_repo(move |r| {
            let mut diff_opts = git2::DiffOptions::new();
            if let Some(ref p) = path {
                diff_opts.pathspec(p);
            }

            let head_tree = match r.head() {
                Ok(head) => {
                    let commit = head.peel_to_commit()?;
                    Some(commit.tree()?)
                }
                Err(_) => None, // no commits yet
            };

            let diff = r.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))?;
            parse_diff(&diff)
        })
        .await
    }

    /// Diff HEAD vs workdir (all uncommitted changes).
    pub async fn head_vs_workdir(repo: &Repository) -> anyhow::Result<Vec<Self>> {
        repo.with_repo(move |r| {
            let head_tree = match r.head() {
                Ok(head) => {
                    let commit = head.peel_to_commit()?;
                    Some(commit.tree()?)
                }
                Err(_) => None,
            };

            let diff = r.diff_tree_to_workdir_with_index(head_tree.as_ref(), None)?;
            parse_diff(&diff)
        })
        .await
    }
}

// ── internal diff parser ──────────────────────────────────────────────────────

/// Convert a `git2::Diff` into our `Vec<FileDiff>` representation.
fn parse_diff(diff: &git2::Diff<'_>) -> anyhow::Result<Vec<FileDiff>> {
    // We collect the full diff by iterating file-by-file using Patch.
    let num_deltas = diff.deltas().count();
    let mut result = Vec::with_capacity(num_deltas);

    for delta_idx in 0..num_deltas {
        let patch = git2::Patch::from_diff(diff, delta_idx)?;
        let patch = match patch {
            Some(p) => p,
            None => continue,
        };

        let delta = patch.delta();

        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        let old_path = {
            let old = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned());
            let new = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().into_owned());
            match (&old, &new) {
                (Some(o), Some(n)) if o != n => Some(o.clone()),
                _ => None,
            }
        };

        let is_binary = delta.new_file().is_binary()
            || delta.old_file().is_binary()
            || delta.flags().contains(git2::DiffFlags::BINARY);

        let stats = patch.line_stats()?;
        let additions = stats.1 as u32;
        let deletions = stats.2 as u32;

        let mut hunks: Vec<DiffHunk> = Vec::new();

        let num_hunks = patch.num_hunks();
        for hunk_idx in 0..num_hunks {
            let (hunk, _) = patch.hunk(hunk_idx)?;

            let header = std::str::from_utf8(hunk.header())
                .unwrap_or("")
                .trim_end()
                .to_string();

            let old_start = hunk.old_start();
            let old_lines = hunk.old_lines();
            let new_start = hunk.new_start();
            let new_lines = hunk.new_lines();

            let num_lines = patch.num_lines_in_hunk(hunk_idx)?;
            let mut lines = Vec::with_capacity(num_lines);

            for line_idx in 0..num_lines {
                let line = patch.line_in_hunk(hunk_idx, line_idx)?;

                let kind = match line.origin() {
                    '+' => DiffLineKind::Added,
                    '-' => DiffLineKind::Removed,
                    '\\' => DiffLineKind::NoNewline,
                    _ => DiffLineKind::Context,
                };

                let content = std::str::from_utf8(line.content())
                    .unwrap_or("")
                    .to_string();

                lines.push(DiffLine {
                    kind,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                    content,
                });
            }

            hunks.push(DiffHunk {
                header,
                old_start,
                old_lines,
                new_start,
                new_lines,
                lines,
            });
        }

        result.push(FileDiff {
            path,
            old_path,
            hunks,
            is_binary,
            additions,
            deletions,
        });
    }

    Ok(result)
}
