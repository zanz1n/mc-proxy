use crate::repository::RepositoryError;
use serde::{Deserialize, Serialize};

pub mod handler;
pub mod server;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command decode failed: {0}")]
    CommandDecodeError(serde_json::Error),
    #[error("Command encode failed: {0}")]
    CommandEncodeError(serde_json::Error),
    #[error("Internal repository error: {0}")]
    RepositoryError(#[from] RepositoryError),

    #[error("The provided duration is invalid")]
    InvalidDuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum CommandResult<T> {
    Success(T),
    Error(ErrorMessage),
}

impl<T, E> From<Result<T, E>> for CommandResult<T>
where
    E: ToString,
{
    #[inline]
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(v) => Self::Success(v),
            Err(err) => Self::Error(ErrorMessage::from(err)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorMessage {
    pub error: String,
}

impl<T: ToString> From<T> for ErrorMessage {
    #[inline]
    fn from(value: T) -> Self {
        Self {
            error: value.to_string(),
        }
    }
}
