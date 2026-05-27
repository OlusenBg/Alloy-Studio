//! SHA-256 digest cache for file content change detection.

use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A thread-safe cache mapping file paths to their last-computed SHA-256 hex digest.
pub struct DigestCache {
    inner: DashMap<PathBuf, String>,
}

impl DigestCache {
    /// Create a new, empty digest cache.
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    /// Compute the SHA-256 digest of `path`, store it in the cache, and return
    /// the hex-encoded string.
    pub async fn compute(&self, path: &Path) -> anyhow::Result<String> {
        let content = tokio::fs::read(path)
            .await
            .map_err(|e| anyhow::anyhow!("digest read {}: {}", path.display(), e))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let digest = hex::encode(hasher.finalize());

        self.inner.insert(path.to_path_buf(), digest.clone());
        Ok(digest)
    }

    /// Return the cached digest for `path`, or `None` if it has not been computed.
    pub fn get(&self, path: &Path) -> Option<String> {
        self.inner.get(path).map(|v| v.clone())
    }

    /// Remove the cached digest for `path`.
    ///
    /// Returns `true` if an entry existed and was removed, `false` otherwise.
    pub fn invalidate(&self, path: &Path) -> bool {
        self.inner.remove(path).is_some()
    }

    /// Check whether the file at `path` has changed since the last call.
    ///
    /// Reads the file, computes its digest, compares it with the cached value,
    /// and updates the cache.  Returns `true` if the digest differs (or if no
    /// cached value existed), `false` if the file is unchanged.
    pub async fn has_changed(&self, path: &Path) -> anyhow::Result<bool> {
        let previous = self.get(path);
        let current = self.compute(path).await?;
        Ok(previous.as_deref() != Some(current.as_str()))
    }

    /// Remove all entries from the cache.
    pub fn clear(&self) {
        self.inner.clear();
    }
}

impl Default for DigestCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_compute_and_get() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, b"hello").unwrap();

        let cache = DigestCache::new();
        let digest = cache.compute(&path).await.unwrap();
        assert_eq!(digest.len(), 64); // SHA-256 hex string length
        assert_eq!(cache.get(&path).as_deref(), Some(digest.as_str()));
    }

    #[tokio::test]
    async fn test_invalidate() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, b"data").unwrap();

        let cache = DigestCache::new();
        cache.compute(&path).await.unwrap();
        assert!(cache.get(&path).is_some());

        let removed = cache.invalidate(&path);
        assert!(removed);
        assert!(cache.get(&path).is_none());

        // Invalidating again returns false.
        assert!(!cache.invalidate(&path));
    }

    #[tokio::test]
    async fn test_has_changed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, b"v1").unwrap();

        let cache = DigestCache::new();
        // First call: no previous value → reports changed.
        assert!(cache.has_changed(&path).await.unwrap());

        // Second call with same content → unchanged.
        assert!(!cache.has_changed(&path).await.unwrap());

        // Modify the file.
        std::fs::write(&path, b"v2").unwrap();
        assert!(cache.has_changed(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_clear() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, b"x").unwrap();

        let cache = DigestCache::new();
        cache.compute(&path).await.unwrap();
        cache.clear();
        assert!(cache.get(&path).is_none());
    }
}
