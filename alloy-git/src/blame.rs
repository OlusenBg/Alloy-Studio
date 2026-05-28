//! Git blame: map each line of a file to the commit that last touched it.

use serde::{Deserialize, Serialize};

use crate::repo::Repository;

// ── BlameLine ─────────────────────────────────────────────────────────────────

/// Blame information for a single source line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    /// 1-based line number.
    pub line_no: u32,
    /// Short 7-character commit hash.
    pub commit_id: String,
    pub author: String,
    pub email: String,
    /// Unix timestamp of the commit.
    pub timestamp: i64,
    /// First line of the commit message.
    pub summary: String,
}

// ── GitBlame ─────────────────────────────────────────────────────────────────

/// Blame output for an entire file.
pub struct GitBlame(pub Vec<BlameLine>);

impl GitBlame {
    /// Compute blame for `path` (repo-relative).
    pub async fn for_file(repo: &Repository, path: &str) -> anyhow::Result<Self> {
        let path = path.to_string();
        let lines = repo
            .with_repo(move |r| {
                let blame = r.blame_file(std::path::Path::new(&path), None)?;

                let mut lines: Vec<BlameLine> = Vec::new();
                let mut current_line: u32 = 1;

                for hunk in blame.iter() {
                    let sig = hunk.final_signature();
                    let oid = hunk.final_commit_id();

                    // Short hash (7 chars).
                    let commit_id = format!("{:.7}", oid);

                    let author = sig.name().unwrap_or("Unknown").to_string();

                    let email = sig.email().unwrap_or("").to_string();

                    let timestamp = sig.when().seconds();

                    // Retrieve the commit message summary.
                    let summary = r
                        .find_commit(oid)
                        .ok()
                        .and_then(|c| c.summary().map(|s| s.to_string()))
                        .unwrap_or_default();

                    let lines_in_hunk = hunk.lines_in_hunk() as u32;
                    for _ in 0..lines_in_hunk {
                        lines.push(BlameLine {
                            line_no: current_line,
                            commit_id: commit_id.clone(),
                            author: author.clone(),
                            email: email.clone(),
                            timestamp,
                            summary: summary.clone(),
                        });
                        current_line += 1;
                    }
                }

                Ok(lines)
            })
            .await?;

        Ok(GitBlame(lines))
    }
}
