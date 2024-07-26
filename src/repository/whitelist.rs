use super::{kv::KeyValueRepository, private::SealedRepository, RepositoryError};
use chrono::Utc;
use futures_util::TryStreamExt;
use sqlx::{
    database::HasArguments, ColumnIndex, Database, Decode, Encode, Executor, FromRow,
    IntoArguments, Pool, Row, Type,
};
use std::future::Future;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum WhitelistResult {
    Changed,
    Unchanged,
}

impl WhitelistResult {
    #[inline]
    pub fn is_changed(&self) -> bool {
        match self {
            WhitelistResult::Changed => true,
            WhitelistResult::Unchanged => false,
        }
    }
}

pub trait WhitelistRepository: SealedRepository {
    fn add(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<WhitelistResult, RepositoryError>> + Send;

    fn is_enabled(&self) -> impl Future<Output = Result<bool, RepositoryError>> + Send;

    fn set_enabled(
        &self,
        enabled: bool,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn is_whitelisted(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;

    fn remove(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<WhitelistResult, RepositoryError>> + Send;

    fn get_all(&self) -> impl Future<Output = Result<Vec<String>, RepositoryError>> + Send;
}

struct WhitelistRow {
    username: String,
    created_at: i64,
}

impl<'r, R: Row> FromRow<'r, R> for WhitelistRow
where
    &'static str: ColumnIndex<R>,
    i64: Decode<'r, R::Database> + Type<R::Database>,
    String: Decode<'r, R::Database> + Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let data = WhitelistRow {
            username: row.try_get("username")?,
            created_at: row.try_get("created_at")?,
        };

        Ok(data)
    }
}

pub struct SqlxWhitelistRepository<DB: Database, KV> {
    db: Pool<DB>,
    kv: KV,
}

impl<DB: Database, KV: Clone> Clone for SqlxWhitelistRepository<DB, KV> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            kv: self.kv.clone(),
        }
    }
}

impl<DB: Database, KV: KeyValueRepository> SqlxWhitelistRepository<DB, KV> {
    #[inline]
    pub fn new(db: Pool<DB>, kv: KV) -> Self {
        Self { db, kv }
    }
}

impl<DB: Database, KV: Send + Sync> SealedRepository for SqlxWhitelistRepository<DB, KV> {}

impl<DB, KV> WhitelistRepository for SqlxWhitelistRepository<DB, KV>
where
    DB: Database,
    KV: KeyValueRepository,
    for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    for<'a> &'a Pool<DB>: Executor<'a, Database = DB>,

    for<'r> WhitelistRow: FromRow<'r, DB::Row>,

    for<'e> i64: Encode<'e, DB> + Type<DB>,
    for<'e> &'e str: Encode<'e, DB> + Type<DB>,
{
    async fn add(&self, username: &str) -> Result<WhitelistResult, RepositoryError> {
        let now = Utc::now();

        if !self.is_whitelisted(username).await? {
            sqlx::query("INSERT INTO whitelist (username, created_at) VALUES ($1, $2)")
                .bind(username)
                .bind(now.timestamp_millis())
                .execute(&self.db)
                .await
                .map(|_| ())
                .map_err(|error| {
                    tracing::error!(%error, "Failed to create whitelist registry: sqlx error");
                    error
                })?;

            Ok(WhitelistResult::Changed)
        } else {
            Ok(WhitelistResult::Unchanged)
        }
    }

    async fn is_enabled(&self) -> Result<bool, RepositoryError> {
        self.kv
            .get("whitelist.enabled")
            .await
            .map(|v| v.map_or(false, |v| v == "true"))
            .map_err(Into::into)
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
        let value = if enabled { "true" } else { "false" };

        self.kv
            .set("whitelist.enabled", value)
            .await
            .map_err(Into::into)
    }

    async fn is_whitelisted(&self, username: &str) -> Result<bool, RepositoryError> {
        sqlx::query("SELECT created_at FROM whitelist WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.db)
            .await
            .map(|v| v.is_some())
            .map_err(|error| {
                tracing::error!(%error, "Failed to get whitelist registry: sqlx error");
                error.into()
            })
    }

    async fn remove(&self, username: &str) -> Result<WhitelistResult, RepositoryError> {
        sqlx::query("DELETE FROM whitelist WHERE username = $1 RETURNING *")
            .bind(username)
            .fetch_optional(&self.db)
            .await
            .map(|v| match v {
                Some(_) => WhitelistResult::Changed,
                None => WhitelistResult::Unchanged,
            })
            .map_err(|error| {
                tracing::error!(%error, "Failed to delete whitelist registry: sqlx error");
                error.into()
            })
    }

    async fn get_all(&self) -> Result<Vec<String>, RepositoryError> {
        sqlx::query_as("SELECT * FROM whitelist")
            .fetch(&self.db)
            .try_filter_map(|v: WhitelistRow| async move { Ok(Some(v.username)) })
            .try_collect()
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to get all whitelist registries: sqlx error");
                error.into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::SqlxWhitelistRepository;
    use crate::repository::{
        kv::SqlxKeyValueRepository,
        whitelist::{WhitelistRepository, WhitelistResult},
    };
    use sqlx::{migrate, Sqlite, SqlitePool};
    use std::collections::HashSet;
    use uuid::Uuid;

    async fn get_repository() -> SqlxWhitelistRepository<Sqlite, SqlxKeyValueRepository<Sqlite>> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        migrate!().run(&pool).await.unwrap();

        let key_value = SqlxKeyValueRepository::new(pool.clone());

        SqlxWhitelistRepository::new(pool, key_value)
    }

    fn rand_string() -> String {
        Uuid::new_v4().to_string()
    }

    #[tokio::test]
    async fn test_add_whitelist() {
        let repo = get_repository().await;

        let username = rand_string();

        let result = repo.add(&username).await.unwrap();
        assert_eq!(result, WhitelistResult::Changed);

        let result = repo.is_whitelisted(&username).await.unwrap();
        assert_eq!(result, true);

        let result = repo.add(&username).await.unwrap();
        assert_eq!(result, WhitelistResult::Unchanged);
    }

    #[tokio::test]
    async fn test_enabling_whitelist() {
        let repo = get_repository().await;

        let result = repo.is_enabled().await.unwrap();
        assert_eq!(result, false);

        repo.set_enabled(false).await.unwrap();
        let result = repo.is_enabled().await.unwrap();
        assert_eq!(result, false);

        repo.set_enabled(true).await.unwrap();

        let result = repo.is_enabled().await.unwrap();
        assert_eq!(result, true);

        repo.set_enabled(false).await.unwrap();
        let result = repo.is_enabled().await.unwrap();
        assert_eq!(result, false);
    }

    #[tokio::test]
    async fn test_remove_whitelist() {
        let repo = get_repository().await;

        let username = rand_string();

        let result = repo.remove(&username).await.unwrap();
        assert_eq!(result, WhitelistResult::Unchanged);

        let result = repo.add(&username).await.unwrap();
        assert_eq!(result, WhitelistResult::Changed);

        let result = repo.remove(&username).await.unwrap();
        assert_eq!(result, WhitelistResult::Changed);

        let result = repo.remove(&username).await.unwrap();
        assert_eq!(result, WhitelistResult::Unchanged);

        let result = repo.is_whitelisted(&username).await.unwrap();
        assert_eq!(result, false);
    }

    #[tokio::test]
    async fn test_get_all_whitelist() {
        let repo = get_repository().await;

        let mut all_adds = HashSet::new();

        for _ in 0..10 {
            let username = rand_string();

            let result = repo.add(&username).await.unwrap();
            assert_eq!(result, WhitelistResult::Changed);

            all_adds.insert(username);
        }

        for username in repo.get_all().await.unwrap() {
            assert!(all_adds.remove(&username));
        }

        assert_eq!(all_adds.len(), 0);
    }
}
