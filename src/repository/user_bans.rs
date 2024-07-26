use super::RepositoryError;
use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use sqlx::{
    database::HasArguments, prelude::FromRow, ColumnIndex, Database, Decode, Encode, Executor,
    IntoArguments, Pool, Row, Type,
};
use std::{future::Future, time::Duration};

#[derive(Debug, Clone)]
pub struct UserBanData {
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub expiration: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

pub trait UserBansRepository: Clone + Send + Sync {
    fn add_ban(
        &self,
        username: &str,
        expiration: Option<Duration>,
        reason: Option<String>,
    ) -> impl Future<Output = Result<UserBanData, RepositoryError>> + Send;

    fn is_banned(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<Option<UserBanData>, RepositoryError>> + Send;

    fn remove_ban(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<Option<UserBanData>, RepositoryError>> + Send;

    fn get_bans(&self) -> impl Future<Output = Result<Vec<UserBanData>, RepositoryError>> + Send;
}

impl<'r, R: Row> FromRow<'r, R> for UserBanData
where
    &'static str: ColumnIndex<R>,
    String: Decode<'r, R::Database> + Type<R::Database>,
    DateTime<Utc>: Decode<'r, R::Database> + Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let data = Self {
            username: row.try_get("username")?,
            created_at: row.try_get("created_at")?,
            expiration: row.try_get("expiration")?,
            reason: row.try_get("reason")?,
        };

        Ok(data)
    }
}

pub struct SqlxUserBansRepository<DB: Database> {
    db: Pool<DB>,
}

impl<DB: Database> Clone for SqlxUserBansRepository<DB> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

impl<DB: Database> SqlxUserBansRepository<DB> {
    #[inline]
    pub fn new(db: Pool<DB>) -> Self {
        Self { db }
    }
}

impl<DB> UserBansRepository for SqlxUserBansRepository<DB>
where
    DB: Database,
    for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    for<'a> &'a Pool<DB>: Executor<'a, Database = DB>,

    for<'r> UserBanData: FromRow<'r, DB::Row>,

    for<'e> DateTime<Utc>: Encode<'e, DB> + Type<DB>,
    for<'e> Option<DateTime<Utc>>: Encode<'e, DB> + Type<DB>,
    for<'e> &'e str: Encode<'e, DB> + Type<DB>,
    for<'e> Option<String>: Encode<'e, DB> + Type<DB>,
{
    async fn add_ban(
        &self,
        username: &str,
        expiration: Option<Duration>,
        reason: Option<String>,
    ) -> Result<UserBanData, RepositoryError> {
        let now = Utc::now();
        let exp = expiration.map(|exp| now + exp);

        if let Some(data) = self.is_banned(username).await? {
            if exp != data.expiration || data.reason != reason {
                let row = sqlx::query_as(
                    "UPDATE user_bans \
                    SET expiration = $1, reason = $2 \
                    WHERE username = $3 \
                    RETURNING*",
                )
                .bind(exp)
                .bind(reason)
                .bind(username)
                .fetch_one(&self.db)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "Failed to update user ban registry: sqlx error");
                    error
                })?;

                Ok(row)
            } else {
                Ok(data)
            }
        } else {
            let row = sqlx::query_as(
                "INSERT INTO user_bans \
                (username, created_at, expiration, reason) \
                VALUES ($1, $2, $3, $4) \
                RETURNING *",
            )
            .bind(username)
            .bind(now)
            .bind(exp)
            .bind(reason)
            .fetch_one(&self.db)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to create user ban registry: sqlx error");
                error
            })?;

            Ok(row)
        }
    }

    async fn is_banned(&self, username: &str) -> Result<Option<UserBanData>, RepositoryError> {
        let now = Utc::now();

        let row: Option<UserBanData> =
            sqlx::query_as("SELECT * FROM user_bans WHERE username = $1")
                .bind(&username)
                .fetch_optional(&self.db)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "Failed to get user ban registry: sqlx error");
                    error
                })?;

        if let Some(row) = row {
            if matches!(row.expiration, Some(expiration) if now > expiration) {
                let _ = sqlx::query("DELETE FROM user_bans WHERE username = $1")
                    .bind(username)
                    .execute(&self.db)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Failed to delete expired user ban registry: sqlx error");
                    });

                Ok(None)
            } else {
                Ok(Some(row))
            }
        } else {
            Ok(None)
        }
    }

    async fn remove_ban(&self, username: &str) -> Result<Option<UserBanData>, RepositoryError> {
        sqlx::query_as("DELETE FROM user_bans WHERE username = $1 RETURNING *")
            .bind(username)
            .fetch_optional(&self.db)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to delete user ban registry: sqlx error");
                error.into()
            })
    }

    async fn get_bans(&self) -> Result<Vec<UserBanData>, RepositoryError> {
        sqlx::query_as("SELECT * FROM user_bans")
            .fetch(&self.db)
            .try_collect()
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to get all user ban registries: sqlx error");
                error.into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{SqlxUserBansRepository, UserBansRepository};
    use chrono::Utc;
    use sqlx::{migrate, Sqlite, SqlitePool};
    use std::{collections::HashSet, time::Duration};
    use tokio::time::sleep;
    use uuid::Uuid;

    async fn get_repository() -> SqlxUserBansRepository<Sqlite> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        migrate!().run(&pool).await.unwrap();

        SqlxUserBansRepository::new(pool)
    }

    fn rand_string() -> String {
        Uuid::new_v4().to_string()
    }

    #[tokio::test]
    async fn test_add_ban() {
        let repo = get_repository().await;

        let username = rand_string();
        let reason = rand_string();

        let now = Utc::now();
        repo.add_ban(&username, None, Some(reason.clone()))
            .await
            .unwrap();

        let ban = repo
            .is_banned(&username)
            .await
            .unwrap()
            .expect("The added ban was not registrered properly");

        assert_eq!(ban.username, username);
        assert_eq!(ban.reason.unwrap(), reason);
        assert_eq!(ban.created_at.timestamp(), now.timestamp());
    }

    #[tokio::test]
    async fn test_remove_ban() {
        let repo = get_repository().await;

        let username = rand_string();

        let result = repo.remove_ban(&username).await.unwrap();
        assert!(matches!(result, None));

        repo.add_ban(&username, None, None).await.unwrap();

        let result = repo.remove_ban(&username).await.unwrap();
        assert!(matches!(result, Some(_)));

        let result = repo.remove_ban(&username).await.unwrap();
        assert!(matches!(result, None));

        let result = repo.is_banned(&username).await.unwrap();
        assert!(matches!(result, None));
    }

    #[tokio::test]
    async fn test_ban_expiration() {
        let repo = get_repository().await;

        let username = rand_string();

        repo.add_ban(&username, Some(Duration::from_millis(100)), None)
            .await
            .unwrap();

        let result = repo.is_banned(&username).await.unwrap();
        assert!(matches!(result, Some(_)));

        sleep(Duration::from_millis(200)).await;
        let result = repo.is_banned(&username).await.unwrap();
        assert!(matches!(result, None));
    }

    #[tokio::test]
    async fn test_get_all_bans() {
        let repo = get_repository().await;

        let mut all_adds = HashSet::new();

        for _ in 0..10 {
            let username = rand_string();
            all_adds.insert(username.clone());

            repo.add_ban(&username, None, None).await.unwrap();
        }

        for data in repo.get_bans().await.unwrap() {
            assert!(all_adds.remove(&data.username));
        }

        assert_eq!(all_adds.len(), 0);
    }
}
