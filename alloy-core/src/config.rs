//! Editor and FTC configuration, with file watching support.

use anyhow::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// LineEnding
// ---------------------------------------------------------------------------

/// The line-ending convention used when writing files.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// Unix-style LF (`\n`).
    #[default]
    Lf,
    /// Windows-style CRLF (`\r\n`).
    CrLf,
    /// Classic Mac CR (`\r`).
    Cr,
}

impl LineEnding {
    /// Return the byte sequence for this line ending.
    pub fn as_str(&self) -> &str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }
}

// ---------------------------------------------------------------------------
// EditorConfig
// ---------------------------------------------------------------------------

/// Visual and behaviour settings for the text editor.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EditorConfig {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub trim_trailing_whitespace: bool,
    pub font_family: String,
    pub font_size: f32,
    pub line_ending: LineEnding,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub show_minimap: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            font_family: "JetBrains Mono".to_owned(),
            font_size: 14.0,
            line_ending: LineEnding::Lf,
            word_wrap: false,
            show_line_numbers: true,
            show_minimap: true,
        }
    }
}

// ---------------------------------------------------------------------------
// FtcConfig
// ---------------------------------------------------------------------------

/// FTC-SDK-specific settings.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FtcConfig {
    pub sdk_path: Option<PathBuf>,
    pub jdtls_heap_mb: u32,
    pub telemetry_port: u16,
    pub auto_repair: bool,
    pub team_number: Option<u32>,
}

impl Default for FtcConfig {
    fn default() -> Self {
        Self {
            sdk_path: None,
            jdtls_heap_mb: 512,
            telemetry_port: 5800,
            auto_repair: true,
            team_number: None,
        }
    }
}

// ---------------------------------------------------------------------------
// GitConfig
// ---------------------------------------------------------------------------

/// Git-integration settings.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GitConfig {
    pub auto_stage: bool,
    pub sign_commits: bool,
    pub default_branch: String,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            auto_stage: false,
            sign_commits: false,
            default_branch: "main".to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// AlloyConfig
// ---------------------------------------------------------------------------

/// Top-level configuration for Alloy Studio, serialised as TOML.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct AlloyConfig {
    pub editor: EditorConfig,
    pub ftc: FtcConfig,
    pub git: GitConfig,
}

impl AlloyConfig {
    // --- I/O ----------------------------------------------------------------

    /// Load configuration from a TOML file.  Missing keys are filled in with
    /// their defaults through the `Default` implementation.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        let cfg: AlloyConfig = toml::from_str(&text)
            .with_context(|| format!("parsing config from {}", path.display()))?;
        Ok(cfg)
    }

    /// Serialise the configuration to a TOML file, creating any parent
    /// directories as needed.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).context("serialising AlloyConfig to TOML")?;
        std::fs::write(path, text.as_bytes())
            .with_context(|| format!("writing config to {}", path.display()))?;
        Ok(())
    }

    // --- Paths --------------------------------------------------------------

    /// Return the platform-appropriate configuration directory.
    ///
    /// | Platform | Path                                          |
    /// |----------|-----------------------------------------------|
    /// | Linux    | `~/.config/alloy-studio/`                     |
    /// | macOS    | `~/Library/Application Support/alloy-studio/` |
    /// | Windows  | `%APPDATA%\alloy-studio\`                     |
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("alloy-studio")
    }

    /// Return the default path for the main configuration file.
    pub fn default_config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }
}

// ---------------------------------------------------------------------------
// ConfigWatcher
// ---------------------------------------------------------------------------

/// Watches the configuration file for changes, hot-reloads it, and broadcasts
/// the new config through a [`crossbeam_channel`] channel.
pub struct ConfigWatcher {
    config: Arc<parking_lot::RwLock<AlloyConfig>>,
    path: PathBuf,
    /// Keep the watcher alive for its lifetime.
    _watcher: RecommendedWatcher,
    change_tx: crossbeam_channel::Sender<AlloyConfig>,
}

impl ConfigWatcher {
    /// Create a watcher for `path`.
    ///
    /// If the file does not yet exist the default config is used.  Returns
    /// `(watcher, receiver)` — the caller can poll the receiver to learn about
    /// config changes.
    pub fn new(path: PathBuf) -> anyhow::Result<(Self, crossbeam_channel::Receiver<AlloyConfig>)> {
        // Load initial config (or use default).
        let initial = AlloyConfig::load(&path).unwrap_or_default();
        let config = Arc::new(parking_lot::RwLock::new(initial));

        let (change_tx, change_rx) = crossbeam_channel::unbounded::<AlloyConfig>();

        // Clones for the closure.
        let config_clone = Arc::clone(&config);
        let tx_clone = change_tx.clone();
        let path_clone = path.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let event = match res {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("config watcher error: {err}");
                    return;
                }
            };

            // Only react to file-modification events.
            let is_modify = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if !is_modify {
                return;
            }

            match AlloyConfig::load(&path_clone) {
                Ok(new_cfg) => {
                    *config_clone.write() = new_cfg.clone();
                    let _ = tx_clone.send(new_cfg);
                }
                Err(err) => {
                    tracing::warn!("failed to reload config: {err}");
                }
            }
        })?;

        // Watch the parent directory so we catch atomic writes (rename).
        let watch_path = path.parent().unwrap_or(Path::new("."));
        watcher.watch(watch_path, RecursiveMode::NonRecursive)?;

        Ok((
            Self {
                config,
                path,
                _watcher: watcher,
                change_tx,
            },
            change_rx,
        ))
    }

    /// Return a snapshot of the current configuration.
    pub fn get(&self) -> AlloyConfig {
        self.config.read().clone()
    }

    /// Persist `new_config` to disk and broadcast it to all receivers.
    pub fn update(&self, new_config: AlloyConfig) -> anyhow::Result<()> {
        new_config.save(&self.path)?;
        *self.config.write() = new_config.clone();
        // Best-effort send — it's fine if no receiver is listening yet.
        let _ = self.change_tx.send(new_config);
        Ok(())
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
    fn test_default_config_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let cfg = AlloyConfig::default();
        cfg.save(&path).unwrap();

        let loaded = AlloyConfig::load(&path).unwrap();
        assert_eq!(loaded.editor.tab_size, 4);
        assert!(loaded.editor.insert_spaces);
        assert_eq!(loaded.ftc.jdtls_heap_mb, 512);
        assert_eq!(loaded.git.default_branch, "main");
    }

    #[test]
    fn test_line_ending_as_str() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
        assert_eq!(LineEnding::Cr.as_str(), "\r");
    }

    #[test]
    fn test_config_dir_is_not_empty() {
        let dir = AlloyConfig::config_dir();
        assert!(!dir.as_os_str().is_empty());
    }
}
