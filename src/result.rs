use std::fmt::Debug;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Parse failed: {0}")]
    ParseFailed(#[from] object::Error),

    #[error("{0}")]
    NotFound(String),
}

pub type ResolverResult<T> = Result<T, ResolverError>;
