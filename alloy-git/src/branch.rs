//! Git branch listing and management.

use serde::{Deserialize, Serialize};

use crate::repo::Repository;

// ── Branch ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_remote: bool,
    pub is_head: bool,
    pub upstream: Option<String>,
    /// Full OID hex string of the branch tip.
    pub commit_id: String,
    /// First line of the commit message at the branch tip.
    pub commit_summary: String,
}

// ── BranchList ────────────────────────────────────────────────────────────────

pub struct BranchList(pub Vec<Branch>);

impl BranchList {
    /// Collect all local and remote branches.
    pub async fn collect(repo: &Repository) -> anyhow::Result<Self> {
        let branches = repo
            .with_repo(|r| {
                let mut branches = Vec::new();

                let head_oid = r.head().ok().and_then(|h| h.target());

                let iter = r.branches(None)?;

                for item in iter {
                    let (branch, branch_type) = item?;

                    let name = branch.name()?.unwrap_or("").to_string();
                    let is_remote = matches!(branch_type, git2::BranchType::Remote);

                    let refobj = branch.get().resolve()?;
                    let commit_oid = refobj
                        .target()
                        .ok_or_else(|| anyhow::anyhow!("branch has no target OID"))?;

                    let is_head = Some(commit_oid) == head_oid && !is_remote;

                    let commit = r.find_commit(commit_oid)?;
                    let commit_id = commit_oid.to_string();
                    let commit_summary = commit
                        .summary()
                        .unwrap_or("")
                        .to_string();

                    let upstream = branch
                        .upstream()
                        .ok()
                        .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));

                    branches.push(Branch {
                        name,
                        is_remote,
                        is_head,
                        upstream,
                        commit_id,
                        commit_summary,
                    });
                }

                Ok(branches)
            })
            .await?;

        Ok(BranchList(branches))
    }

    /// Return the currently checked-out branch, if any.
    pub fn current(&self) -> Option<&Branch> {
        self.0.iter().find(|b| b.is_head)
    }
}

// ── operations ────────────────────────────────────────────────────────────────

/// Check out `branch_name`, updating HEAD and the working tree.
pub async fn checkout(repo: &Repository, branch_name: &str) -> anyhow::Result<()> {
    let name = branch_name.to_string();
    repo.with_repo(move |r| {
        let branch = r
            .find_branch(&name, git2::BranchType::Local)
            .map_err(|_| crate::error::GitError::BranchNotFound(name.clone()))?;

        let obj = branch.get().peel(git2::ObjectType::Commit)?;
        r.checkout_tree(&obj, None)?;

        let refname = format!("refs/heads/{name}");
        r.set_head(&refname)?;

        Ok(())
    })
    .await
}

/// Create a new local branch.
///
/// If `from_head` is `true`, the branch is created at HEAD.
pub async fn create_branch(
    repo: &Repository,
    name: &str,
    from_head: bool,
) -> anyhow::Result<()> {
    let name = name.to_string();
    repo.with_repo(move |r| {
        let target_commit = if from_head {
            let head = r.head()?;
            head.peel_to_commit()?
        } else {
            let head = r.head()?;
            head.peel_to_commit()?
        };

        r.branch(&name, &target_commit, false)?;
        Ok(())
    })
    .await
}

/// Delete a local branch.
pub async fn delete_branch(repo: &Repository, name: &str) -> anyhow::Result<()> {
    let name = name.to_string();
    repo.with_repo(move |r| {
        let mut branch = r
            .find_branch(&name, git2::BranchType::Local)
            .map_err(|_| crate::error::GitError::BranchNotFound(name.clone()))?;

        branch.delete()?;
        Ok(())
    })
    .await
}
