//! Workspace — manages a set of open documents and the on-disk file tree.

use crate::document::Document;
use dashmap::DashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// FileEntry
// ---------------------------------------------------------------------------

/// A single entry in the workspace file tree.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    /// Absolute path.
    pub path: PathBuf,
    /// Path relative to the workspace root, using `/` as separator.
    pub relative_path: String,
    /// `true` for directories, `false` for files.
    pub is_dir: bool,
    /// Depth from the workspace root (root itself is depth 0).
    pub depth: usize,
    /// Language identifier for files, `None` for directories.
    pub language_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

/// The top-level workspace, owning open documents and the file-tree index.
pub struct Workspace {
    /// Absolute path to the workspace root directory.
    pub root: PathBuf,
    documents: DashMap<String, Document>,
    file_tree: parking_lot::RwLock<Vec<FileEntry>>,
}

impl Workspace {
    // --- Construction -------------------------------------------------------

    /// Create an empty workspace rooted at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            documents: DashMap::new(),
            file_tree: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Walk up the directory tree from `start` looking for project markers.
    ///
    /// Priority order:
    /// 1. `build.gradle` or `FtcRobotController` directory → that directory.
    /// 2. `.git` directory → that directory.
    /// 3. Else → `start` itself.
    pub fn detect_root(start: &Path) -> PathBuf {
        let mut git_root: Option<PathBuf> = None;

        let mut current = if start.is_dir() {
            start.to_path_buf()
        } else {
            start
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| start.to_path_buf())
        };

        loop {
            // FTC project markers.
            if current.join("build.gradle").exists() || current.join("FtcRobotController").exists()
            {
                return current;
            }

            // Git root (lower priority).
            if git_root.is_none() && current.join(".git").exists() {
                git_root = Some(current.clone());
            }

            match current.parent() {
                Some(p) => current = p.to_path_buf(),
                None => break,
            }
        }

        git_root.unwrap_or_else(|| start.to_path_buf())
    }

    // --- Document management ------------------------------------------------

    /// Open a document from disk and register it.  Returns the document URI.
    pub fn open_document(&self, path: PathBuf) -> anyhow::Result<String> {
        let doc = Document::open(path)?;
        let uri = doc.uri.clone();
        self.documents.insert(uri.clone(), doc);
        Ok(uri)
    }

    /// Register a document whose text was provided by the client (e.g. LSP
    /// `textDocument/didOpen`).  Returns the document URI.
    pub fn open_document_with_text(&self, path: PathBuf, text: String, version: i32) -> String {
        let doc = Document::from_text(path, text, version);
        let uri = doc.uri.clone();
        self.documents.insert(uri.clone(), doc);
        uri
    }

    /// Remove a document from the workspace.  Returns `true` if it was open.
    pub fn close_document(&self, uri: &str) -> bool {
        self.documents.remove(uri).is_some()
    }

    /// Obtain a shared reference to an open document.
    pub fn get_document(
        &self,
        uri: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, String, Document>> {
        self.documents.get(uri)
    }

    /// Obtain an exclusive reference to an open document.
    pub fn get_document_mut(
        &self,
        uri: &str,
    ) -> Option<dashmap::mapref::one::RefMut<'_, String, Document>> {
        self.documents.get_mut(uri)
    }

    /// Return the URIs of all currently open documents.
    pub fn all_uris(&self) -> Vec<String> {
        self.documents.iter().map(|e| e.key().clone()).collect()
    }

    // --- File tree ----------------------------------------------------------

    /// Directories/files to skip when building the file tree.
    fn is_ignored(name: &str) -> bool {
        matches!(
            name,
            ".git" | "build" | ".gradle" | ".idea" | ".DS_Store" | "node_modules" | "__pycache__"
        )
    }

    /// Rebuild the file-tree index by walking the workspace root.
    ///
    /// Skips `.git`, `build/`, and `.gradle/` directories.
    pub fn refresh_file_tree(&self) -> anyhow::Result<()> {
        use walkdir::WalkDir;

        let root = &self.root;
        let mut entries: Vec<FileEntry> = Vec::new();

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip ignored directory names.
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        return !Self::is_ignored(name);
                    }
                }
                true
            });

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("file tree walk error: {err}");
                    continue;
                }
            };

            let path = entry.path().to_path_buf();
            let is_dir = entry.file_type().is_dir();
            let depth = entry.depth();

            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let language_id = if is_dir {
                None
            } else {
                Some(Document::language_id_for_path(&path).to_owned())
            };

            entries.push(FileEntry {
                path,
                relative_path,
                is_dir,
                depth,
                language_id,
            });
        }

        *self.file_tree.write() = entries;
        Ok(())
    }

    /// Return a snapshot of the current file tree.
    pub fn file_tree(&self) -> Vec<FileEntry> {
        self.file_tree.read().clone()
    }

    // --- FTC detection ------------------------------------------------------

    /// Returns `true` when the workspace looks like an FTC project.
    ///
    /// Heuristic: the root must have a `build.gradle` file **and** contain a
    /// `TeamCode` directory (the standard FTC app module).
    pub fn is_ftc_project(&self) -> bool {
        self.root.join("build.gradle").exists() && self.root.join("TeamCode").is_dir()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_close_document() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("Foo.java");
        std::fs::write(&path, b"class Foo {}").unwrap();

        let ws = Workspace::new(tmp.path().to_path_buf());
        let uri = ws.open_document(path).unwrap();
        assert!(ws.get_document(&uri).is_some());

        let removed = ws.close_document(&uri);
        assert!(removed);
        assert!(ws.get_document(&uri).is_none());
    }

    #[test]
    fn test_open_document_with_text() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("Main.java");
        let ws = Workspace::new(tmp.path().to_path_buf());
        let uri = ws.open_document_with_text(path, "hello".into(), 1);
        let doc = ws.get_document(&uri).unwrap();
        assert_eq!(doc.buffer.to_string(), "hello");
    }

    #[test]
    fn test_refresh_file_tree() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Main.java"), b"").unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src").join("Util.java"), b"").unwrap();

        let ws = Workspace::new(tmp.path().to_path_buf());
        ws.refresh_file_tree().unwrap();
        let tree = ws.file_tree();
        assert!(tree.iter().any(|e| e.relative_path.contains("Main.java")));
        assert!(tree.iter().any(|e| e.relative_path.contains("Util.java")));
    }

    #[test]
    fn test_detect_root_finds_build_gradle() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("build.gradle"), b"").unwrap();
        let sub = tmp.path().join("TeamCode").join("src");
        std::fs::create_dir_all(&sub).unwrap();

        let root = Workspace::detect_root(&sub);
        // The root should be the directory containing build.gradle
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn test_is_ftc_project_false() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path().to_path_buf());
        assert!(!ws.is_ftc_project());
    }

    #[test]
    fn test_is_ftc_project_true() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("build.gradle"), b"").unwrap();
        std::fs::create_dir(tmp.path().join("TeamCode")).unwrap();
        let ws = Workspace::new(tmp.path().to_path_buf());
        assert!(ws.is_ftc_project());
    }
}
