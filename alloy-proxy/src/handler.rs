//! Request handler — maps incoming alloy-rpc requests to filesystem operations.

use std::path::Path;
use std::sync::Arc;

use alloy_rpc::types::{DirEntry, FileStat, FileEvent};
use crossbeam_channel::{Receiver, Sender};

use crate::digest::DigestCache;
use crate::watcher::FileWatcher;
use crate::{fs_ops};

/// Handles incoming proxy requests by dispatching to filesystem operations.
///
/// Internally owns a [`DigestCache`] for change detection and a
/// [`FileWatcher`] for path-watch management.
pub struct RequestHandler {
    digest_cache: Arc<DigestCache>,
    watcher: Arc<FileWatcher>,
    file_event_rx: Receiver<FileEvent>,
}

impl RequestHandler {
    /// Construct a new `RequestHandler`, returning it along with a receiver
    /// for file-system events.
    ///
    /// The server uses the returned receiver to forward events as
    /// `Notification::FileChanged` to connected clients.
    pub fn new() -> anyhow::Result<(Self, Receiver<FileEvent>)> {
        let (tx, rx): (Sender<FileEvent>, Receiver<FileEvent>) =
            crossbeam_channel::unbounded();

        let watcher = Arc::new(FileWatcher::new(tx)?);
        let digest_cache = Arc::new(DigestCache::new());

        // Clone the receiver so the server can also poll it.
        let rx_clone = rx.clone();

        let handler = Self {
            digest_cache,
            watcher,
            file_event_rx: rx,
        };

        Ok((handler, rx_clone))
    }

    // ── Filesystem operations ─────────────────────────────────────────────────

    /// Read the full byte content of `path`.
    pub async fn handle_read_file(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        fs_ops::read_file(Path::new(path)).await
    }

    /// Write `content` to `path`, creating parent directories as needed.
    ///
    /// After a successful write the digest cache entry for `path` is
    /// invalidated so the next call to `has_changed` will see the new content.
    pub async fn handle_write_file(&self, path: &str, content: Vec<u8>) -> anyhow::Result<()> {
        fs_ops::write_file(Path::new(path), &content).await?;
        self.digest_cache.invalidate(Path::new(path));
        Ok(())
    }

    /// List the entries in a directory, sorted: directories first, then files
    /// (each group sorted alphabetically, case-insensitive).
    pub async fn handle_list_dir(&self, path: &str) -> anyhow::Result<Vec<DirEntry>> {
        fs_ops::list_dir(Path::new(path)).await
    }

    /// Retrieve metadata for a single filesystem path.
    pub async fn handle_stat_file(&self, path: &str) -> anyhow::Result<FileStat> {
        fs_ops::stat_file(Path::new(path)).await
    }

    /// Delete a file or directory (recursively if it is a directory).
    pub async fn handle_delete_file(&self, path: &str) -> anyhow::Result<()> {
        let result = fs_ops::delete_file(Path::new(path)).await;
        self.digest_cache.invalidate(Path::new(path));
        result
    }

    /// Create a directory, optionally including all intermediate parent directories.
    pub async fn handle_create_dir(&self, path: &str, recursive: bool) -> anyhow::Result<()> {
        fs_ops::create_dir(Path::new(path), recursive).await
    }

    // ── File watching ─────────────────────────────────────────────────────────

    /// Begin watching `path` for filesystem changes.
    ///
    /// Events will be forwarded to the receiver returned by [`RequestHandler::new`].
    pub fn handle_watch_path(&self, path: &str) -> anyhow::Result<()> {
        // Default to recursive for directories; non-recursive for individual files.
        let p = Path::new(path);
        let recursive = p.is_dir();
        self.watcher.watch(p, recursive)
    }

    /// Stop watching `path`.
    pub fn handle_unwatch_path(&self, path: &str) -> anyhow::Result<()> {
        self.watcher.unwatch(Path::new(path))
    }

    /// Return a clone of the internal file-event receiver.
    ///
    /// Multiple callers sharing the same receiver will each see every event
    /// (crossbeam channels are MPMC).
    pub fn file_event_receiver(&self) -> Receiver<FileEvent> {
        self.file_event_rx.clone()
    }
}
