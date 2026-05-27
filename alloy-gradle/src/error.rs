/// Errors from the Gradle build subsystem.
#[derive(thiserror::Error, Debug)]
pub enum GradleError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("not an FTC project: {path}")]
    NotFtcProject { path: String },

    #[error("gradlew not found in: {path}")]
    GradlewNotFound { path: String },

    #[error("build failed with exit code {0}")]
    BuildFailed(i32),

    #[error("build was cancelled")]
    Cancelled,

    #[error("parse error: {0}")]
    Parse(String),
}
