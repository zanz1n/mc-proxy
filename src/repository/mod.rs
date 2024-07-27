pub mod ip_bans;
pub mod kv;
pub mod user_bans;
pub mod whitelist;

mod private {
    pub trait SealedRepository: Send + Sync {}
}

pub type DB = sqlx::Sqlite;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Failed to deserialize value: {0}")]
    Json(#[from] serde_json::Error),
}
