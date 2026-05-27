//! Async filesystem operations for the proxy crate.

use alloy_rpc::types::{DirEntry, FileStat};
use std::path::Path;
use std::time::SystemTime;

/// Convert a `SystemTime` to milliseconds since the Unix epoch.
fn system_time_to_ms(t: SystemTime) -> i64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Read the full byte content of a file.
pub async fn read_file(path: &Path) -> anyhow::Result<Vec<u8>> {
    tokio::fs::read(path)
        .await
        .map_err(|e| anyhow::anyhow!("read_file {}: {}", path.display(), e))
}

/// Write byte content to a file, creating parent directories as needed.
pub async fn write_file(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                anyhow::anyhow!("create_dir_all {}: {}", parent.display(), e)
            })?;
        }
    }
    tokio::fs::write(path, content)
        .await
        .map_err(|e| anyhow::anyhow!("write_file {}: {}", path.display(), e))
}

/// List the entries in a directory, sorted: directories first, then files, each
/// group sorted alphabetically (case-insensitive).
pub async fn list_dir(path: &Path) -> anyhow::Result<Vec<DirEntry>> {
    let mut read_dir = tokio::fs::read_dir(path)
        .await
        .map_err(|e| anyhow::anyhow!("list_dir {}: {}", path.display(), e))?;

    let mut entries: Vec<DirEntry> = Vec::new();

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| anyhow::anyhow!("list_dir entry {}: {}", path.display(), e))?
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let entry_path = entry.path().to_string_lossy().into_owned();

        let meta = entry.metadata().await;
        let (is_dir, size, modified_ms) = match meta {
            Ok(m) => {
                let mod_ms = m.modified().map(system_time_to_ms).ok();
                (m.is_dir(), m.len(), mod_ms)
            }
            Err(_) => (false, 0u64, None),
        };

        entries.push(DirEntry {
            name,
            path: entry_path,
            is_dir,
            size,
            modified_ms,
        });
    }

    // Sort: directories first, then files; each group alphabetically by name.
    entries.sort_by(|a, b| {
        match (b.is_dir, a.is_dir) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

/// Retrieve metadata for a single filesystem path.
pub async fn stat_file(path: &Path) -> anyhow::Result<FileStat> {
    let meta = tokio::fs::metadata(path)
        .await
        .map_err(|e| anyhow::anyhow!("stat_file {}: {}", path.display(), e))?;

    let symlink_meta = tokio::fs::symlink_metadata(path).await.ok();
    let is_symlink = symlink_meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    let modified_ms = meta.modified().map(system_time_to_ms).ok();

    Ok(FileStat {
        path: path.to_string_lossy().into_owned(),
        size: meta.len(),
        is_dir: meta.is_dir(),
        is_symlink,
        modified_ms,
    })
}

/// Delete a file or directory (recursively if it is a directory).
pub async fn delete_file(path: &Path) -> anyhow::Result<()> {
    let meta = tokio::fs::metadata(path)
        .await
        .map_err(|e| anyhow::anyhow!("delete_file metadata {}: {}", path.display(), e))?;

    if meta.is_dir() {
        tokio::fs::remove_dir_all(path)
            .await
            .map_err(|e| anyhow::anyhow!("remove_dir_all {}: {}", path.display(), e))
    } else {
        tokio::fs::remove_file(path)
            .await
            .map_err(|e| anyhow::anyhow!("remove_file {}: {}", path.display(), e))
    }
}

/// Create a directory, optionally including all intermediate parent directories.
pub async fn create_dir(path: &Path, recursive: bool) -> anyhow::Result<()> {
    if recursive {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| anyhow::anyhow!("create_dir_all {}: {}", path.display(), e))
    } else {
        tokio::fs::create_dir(path)
            .await
            .map_err(|e| anyhow::anyhow!("create_dir {}: {}", path.display(), e))
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
    async fn test_write_and_read_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        write_file(&path, b"hello world").await.unwrap();
        let content = read_file(&path).await.unwrap();
        assert_eq!(content, b"hello world");
    }

    #[tokio::test]
    async fn test_list_dir_sort_order() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("b.txt"), b"").unwrap();
        std::fs::write(tmp.path().join("a.txt"), b"").unwrap();
        std::fs::create_dir(tmp.path().join("z_dir")).unwrap();

        let entries = list_dir(tmp.path()).await.unwrap();
        // Directory must come first.
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "z_dir");
        // Files sorted alphabetically.
        assert_eq!(entries[1].name, "a.txt");
        assert_eq!(entries[2].name, "b.txt");
    }

    #[tokio::test]
    async fn test_stat_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("stat_me.txt");
        std::fs::write(&path, b"data").unwrap();

        let stat = stat_file(&path).await.unwrap();
        assert_eq!(stat.size, 4);
        assert!(!stat.is_dir);
        assert!(!stat.is_symlink);
    }

    #[tokio::test]
    async fn test_create_and_delete_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("a").join("b").join("c");
        create_dir(&dir, true).await.unwrap();
        assert!(dir.is_dir());
        delete_file(&dir).await.unwrap();
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn test_write_file_creates_parents() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("deep").join("file.txt");
        write_file(&path, b"content").await.unwrap();
        assert!(path.exists());
    }
}
