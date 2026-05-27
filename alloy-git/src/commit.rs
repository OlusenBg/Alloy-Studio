//! Staging, unstaging, committing, and fetching.

use anyhow::Context;

use crate::repo::Repository;

// ── CommitOptions ─────────────────────────────────────────────────────────────

/// Options for creating a commit.
pub struct CommitOptions {
    pub message: String,
    /// Specific paths to stage before committing.  If empty, commits all
    /// already-staged changes.
    pub paths: Vec<String>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

// ── stage_paths ───────────────────────────────────────────────────────────────

/// Add the given paths to the index (stage them).
pub async fn stage_paths(repo: &Repository, paths: &[String]) -> anyhow::Result<()> {
    let paths = paths.to_vec();
    repo.with_repo(move |r| {
        let mut index = r.index()?;
        for path in &paths {
            let p = std::path::Path::new(path);
            if p.exists() {
                index.add_path(p)?;
            } else {
                // File deleted — remove from index.
                index.remove_path(p)?;
            }
        }
        index.write()?;
        Ok(())
    })
    .await
}

// ── unstage_paths ─────────────────────────────────────────────────────────────

/// Remove the given paths from the index (unstage them) by resetting to HEAD.
pub async fn unstage_paths(repo: &Repository, paths: &[String]) -> anyhow::Result<()> {
    let paths = paths.to_vec();
    repo.with_repo(move |r| {
        let head = match r.head() {
            Ok(h) => Some(h.peel_to_commit()?),
            Err(_) => None, // no commits yet — just clear the index entries
        };

        let mut index = r.index()?;

        if let Some(commit) = head {
            // Reset each path in the index to the HEAD tree version.
            for path in &paths {
                let path_str = path.as_str();
                // Find the entry in HEAD tree.
                let tree = commit.tree()?;
                match tree.get_path(std::path::Path::new(path_str)) {
                    Ok(tree_entry) => {
                        let entry = git2::IndexEntry {
                            ctime: git2::IndexTime::new(0, 0),
                            mtime: git2::IndexTime::new(0, 0),
                            dev: 0,
                            ino: 0,
                            mode: tree_entry.filemode() as u32,
                            uid: 0,
                            gid: 0,
                            file_size: 0,
                            id: tree_entry.id(),
                            flags: 0,
                            flags_extended: 0,
                            path: path_str.as_bytes().to_vec(),
                        };
                        index.add(&entry)?;
                    }
                    Err(_) => {
                        // Not in HEAD (newly added) — remove from index.
                        let _ = index.remove_path(std::path::Path::new(path_str));
                    }
                }
            }
        } else {
            for path in &paths {
                let _ = index.remove_path(std::path::Path::new(path.as_str()));
            }
        }

        index.write()?;
        Ok(())
    })
    .await
}

// ── commit ────────────────────────────────────────────────────────────────────

/// Create a commit and return its OID as a hex string.
pub async fn commit(repo: &Repository, options: CommitOptions) -> anyhow::Result<String> {
    repo.with_repo(move |r| {
        // Stage specific paths if requested.
        if !options.paths.is_empty() {
            let mut index = r.index()?;
            for path in &options.paths {
                let p = std::path::Path::new(path);
                if p.exists() {
                    index.add_path(p)?;
                } else {
                    let _ = index.remove_path(p);
                }
            }
            index.write()?;
        }

        let mut index = r.index()?;
        let tree_oid = index.write_tree()?;
        let tree = r.find_tree(tree_oid)?;

        // Determine author / committer.
        let config = r.config()?;

        let author_name = options
            .author_name
            .as_deref()
            .or_else(|| config.get_str("user.name").ok())
            .unwrap_or("Alloy Studio")
            .to_string();

        let author_email = options
            .author_email
            .as_deref()
            .or_else(|| config.get_str("user.email").ok())
            .unwrap_or("alloy@local")
            .to_string();

        let sig = git2::Signature::now(&author_name, &author_email)?;

        // Parent commit (HEAD).
        let parent_commits: Vec<git2::Commit<'_>> = match r.head() {
            Ok(head) => vec![head.peel_to_commit()?],
            Err(_) => vec![], // initial commit
        };

        let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();

        let oid = r.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &options.message,
            &tree,
            &parent_refs,
        )?;

        Ok(oid.to_string())
    })
    .await
}

// ── fetch ─────────────────────────────────────────────────────────────────────

/// Fetch from `remote` with anonymous (unauthenticated) access.
pub async fn fetch(repo: &Repository, remote: &str) -> anyhow::Result<()> {
    let remote = remote.to_string();
    repo.with_repo(move |r| {
        let mut remote_obj = r.find_remote(&remote)
            .or_else(|_| r.remote_anonymous(&remote))
            .context("resolving remote")?;

        // Empty refspecs = use remote's configured refspecs.
        let refspecs: &[&str] = &[];
        let mut fetch_opts = git2::FetchOptions::new();
        // No credentials — anonymous only.
        remote_obj.fetch(refspecs, Some(&mut fetch_opts), None)?;

        Ok(())
    })
    .await
}
