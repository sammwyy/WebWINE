use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("path error: {0}")]
    Path(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("not a directory: {0}")]
    NotADirectory(String),
    #[error("not a file: {0}")]
    NotAFile(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("process not found: pid={0}")]
    ProcessNotFound(u32),
    #[error("pe parse error: {0}")]
    Pe(String),
    #[error("not a pe executable: {0}")]
    NotPe(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

pub type Result<T> = std::result::Result<T, VmError>;
