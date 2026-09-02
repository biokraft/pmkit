use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PmError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("config: {0}")]
    Config(String),
    #[error("refusing to touch {0}: not a path pmkit writes")]
    UnsafePath(PathBuf),
}

pub type Result<T> = std::result::Result<T, PmError>;
