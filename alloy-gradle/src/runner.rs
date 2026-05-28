use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use alloy_rpc::types::BuildEvent;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::parser::BuildOutputParser;
use crate::project::FtcProject;

// ── Task ──────────────────────────────────────────────────────────────────────

/// A Gradle task or set of tasks to run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GradleTask {
    Build,
    AssembleDebug,
    AssembleRelease,
    Clean,
    CleanBuild,
    InstallDebug,
    Test,
    Custom(String),
}

impl GradleTask {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Build => "build",
            Self::AssembleDebug => "assembleDebug",
            Self::AssembleRelease => "assembleRelease",
            Self::Clean => "clean",
            Self::CleanBuild => "clean build",
            Self::InstallDebug => "installDebug",
            Self::Test => "test",
            Self::Custom(s) => s.as_str(),
        }
    }
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Runs Gradle tasks asynchronously, streaming `BuildEvent`s over a broadcast channel.
pub struct GradleRunner {
    project: Arc<FtcProject>,
    cancelled: Arc<AtomicBool>,
}

impl GradleRunner {
    pub fn new(project: Arc<FtcProject>) -> Self {
        Self {
            project,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn a Gradle task and return a receiver for build events plus a join handle.
    ///
    /// The receiver will receive:
    ///   - `BuildEvent::OutputLine` for every stdout/stderr line
    ///   - `BuildEvent::ErrorDetected` when the parser recognises an error
    ///   - `BuildEvent::Finished` when the process exits
    pub fn run(
        &self,
        tasks: &[GradleTask],
    ) -> (
        broadcast::Receiver<BuildEvent>,
        tokio::task::JoinHandle<anyhow::Result<i32>>,
    ) {
        let (tx, rx) = broadcast::channel(256);
        let project = Arc::clone(&self.project);
        let cancelled = Arc::clone(&self.cancelled);
        let tasks: Vec<GradleTask> = tasks.to_vec();

        let handle = tokio::spawn(Self::run_inner(project, tasks, tx, cancelled));
        (rx, handle)
    }

    /// Signal the running build to stop.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    async fn run_inner(
        project: Arc<FtcProject>,
        tasks: Vec<GradleTask>,
        tx: broadcast::Sender<BuildEvent>,
        cancelled: Arc<AtomicBool>,
    ) -> anyhow::Result<i32> {
        // Build the argument list.  CleanBuild needs two separate task names.
        let mut args: Vec<String> = Vec::new();
        for task in &tasks {
            // A single GradleTask::CleanBuild expands to two tokens
            for token in task.as_str().split_whitespace() {
                args.push(token.to_string());
            }
        }
        args.push("--no-daemon".into());
        args.push("--console=plain".into());

        let gradlew = project.gradlew_path();
        info!("Running: {} {}", gradlew.display(), args.join(" "));

        let mut child = tokio::process::Command::new(gradlew)
            .args(&args)
            .current_dir(&project.root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout should be piped");
        let stderr = child.stderr.take().expect("stderr should be piped");

        let mut parser = BuildOutputParser::new();

        // Merge stdout and stderr into a single async line-stream.
        let mut stdout_lines = BufReader::new(stdout).lines();
        let mut stderr_lines = BufReader::new(stderr).lines();

        loop {
            if cancelled.load(Ordering::Relaxed) {
                warn!("build cancelled — killing child process");
                let _ = child.kill().await;
                let _ = tx.send(BuildEvent::Finished {
                    exit_code: -1,
                    errors: parser.finish(),
                });
                return Err(crate::error::GradleError::Cancelled.into());
            }

            // Poll both streams; prefer stdout when both are ready.
            let line = tokio::select! {
                biased;
                line = stdout_lines.next_line() => {
                    match line? {
                        Some(l) => l,
                        None => break,
                    }
                }
                line = stderr_lines.next_line() => {
                    match line? {
                        Some(l) => l,
                        None => {
                            // stderr exhausted; keep draining stdout
                            continue;
                        }
                    }
                }
            };

            debug!("gradle> {}", line);
            let _ = tx.send(BuildEvent::OutputLine(line.clone()));

            if let Some(event) = parser.feed_line(&line) {
                let _ = tx.send(event);
            }
        }

        // Drain the remaining stream that didn't EOF first.
        while let Some(line) = stderr_lines.next_line().await? {
            debug!("gradle(err)> {}", line);
            let _ = tx.send(BuildEvent::OutputLine(line.clone()));
            if let Some(event) = parser.feed_line(&line) {
                let _ = tx.send(event);
            }
        }
        while let Some(line) = stdout_lines.next_line().await? {
            debug!("gradle(out)> {}", line);
            let _ = tx.send(BuildEvent::OutputLine(line.clone()));
            if let Some(event) = parser.feed_line(&line) {
                let _ = tx.send(event);
            }
        }

        let status = child.wait().await?;
        let exit_code = status.code().unwrap_or(-1);
        let errors = parser.finish();

        info!("Gradle finished with exit code {exit_code}");
        let _ = tx.send(BuildEvent::Finished {
            exit_code,
            errors: errors.clone(),
        });

        Ok(exit_code)
    }
}
