use crate::{commands::CommandError, repository::RepositoryError};
use minecraft_protocol::error::{DecodeError, EncodeError};
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to decode package: {0:?}")]
    PacketDecodeError(#[from] DecodeError),
    #[error("Failed to encode package: {0:?}")]
    PacketEncodeError(#[from] EncodeError),

    #[error("Internal repository error: {0}")]
    RepositoryError(#[from] RepositoryError),

    #[error("Command error:")]
    CommandError(#[from] CommandError),
}
