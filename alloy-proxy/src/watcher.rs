//! File system watcher that translates `notify` events into `alloy-rpc` `FileEvent` values.

use alloy_rpc::types::{FileEvent, FileEventKind};
use crossbeam_channel::Sender;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Wraps a `notify` [`RecommendedWatcher`] and keeps track of all watched paths.
///
/// Events are translated into [`FileEvent`] values and sent on the `Sender`
/// supplied at construction time.
pub struct FileWatcher {
    watcher: parking_lot::Mutex<RecommendedWatcher>,
    watched_paths: parking_lot::Mutex<HashSet<PathBuf>>,
}

impl FileWatcher {
    /// Create a new `FileWatcher` that sends [`FileEvent`] values on `tx`.
    pub fn new(tx: Sender<FileEvent>) -> anyhow::Result<Self> {
        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let event = match res {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("file watcher error: {err}");
                    return;
                }
            };

            let kind = match event.kind {
                EventKind::Create(_) => FileEventKind::Created,
                EventKind::Modify(_) => FileEventKind::Changed,
                EventKind::Remove(_) => FileEventKind::Deleted,
                // Access, Other, Any — we don't care.
                _ => return,
            };

            for path in &event.paths {
                let uri = format!("file://{}", path.display());
                let file_event = FileEvent::new(uri, kind);
                if tx.send(file_event).is_err() {
                    // Receiver has been dropped; nothing we can do.
                    break;
                }
            }
        })?;

        Ok(Self {
            watcher: parking_lot::Mutex::new(watcher),
            watched_paths: parking_lot::Mutex::new(HashSet::new()),
        })
    }

    /// Begin watching `path`.
    ///
    /// Pass `recursive = true` to receive events for the entire sub-tree.
    pub fn watch(&self, path: &Path, recursive: bool) -> anyhow::Result<()> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        self.watcher
            .lock()
            .watch(path, mode)
            .map_err(|e| anyhow::anyhow!("watch {}: {e}", path.display()))?;
        self.watched_paths.lock().insert(path.to_path_buf());
        Ok(())
    }

    /// Stop watching `path`.
    pub fn unwatch(&self, path: &Path) -> anyhow::Result<()> {
        self.watcher
            .lock()
            .unwatch(path)
            .map_err(|e| anyhow::anyhow!("unwatch {}: {e}", path.display()))?;
        self.watched_paths.lock().remove(path);
        Ok(())
    }

    /// Return a snapshot of all currently watched paths.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().iter().cloned().collect()
    }
}
