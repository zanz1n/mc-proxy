use super::RepositoryError;
use chrono::Utc;
use sqlx::{
    database::HasArguments, ColumnIndex, Database, Decode, Encode, Executor, FromRow,
    IntoArguments, Pool, Row, Type,
};
use std::{future::Future, time::Duration};

pub trait KeyValueRepository: Send + Sync + Clone {
    fn get_ttl(
        &self,
        key: &str,
        ttl: Option<Duration>,
    ) -> impl Future<Output = Result<Option<String>, RepositoryError>> + Send;

    fn set_ttl(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn delete(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<String>, RepositoryError>> + Send;

    #[inline]
    fn get(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<String>, RepositoryError>> + Send {
        self.get_ttl(key, None)
    }

    #[inline]
    fn set(
        &self,
        key: &str,
        value: &str,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send {
        self.set_ttl(key, value, None)
    }
}

struct KeyValueRow {
    expiration: Option<i64>,
    value: String,
}

impl<'r, R: Row> FromRow<'r, R> for KeyValueRow
where
    &'static str: ColumnIndex<R>,
    String: Decode<'r, R::Database> + Type<R::Database>,
    i64: Decode<'r, R::Database> + Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let data = KeyValueRow {
            expiration: row.try_get("expiration")?,
            value: row.try_get("value")?,
        };

        Ok(data)
    }
}

impl KeyValueRow {
    fn into_string(self) -> String {
        self.value
    }
}

pub struct SqlxKeyValueRepository<DB: Database> {
    db: Pool<DB>,
}

impl<DB: Database> Clone for SqlxKeyValueRepository<DB> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

impl<DB: Database> SqlxKeyValueRepository<DB> {
    #[inline]
    pub fn new(db: Pool<DB>) -> Self {
        Self { db }
    }
}

impl<DB> KeyValueRepository for SqlxKeyValueRepository<DB>
where
    DB: Database,
    for<'a> <DB as HasArguments<'a>>::Arguments: IntoArguments<'a, DB>,
    for<'a> &'a Pool<DB>: Executor<'a, Database = DB>,

    for<'r> KeyValueRow: FromRow<'r, <DB as Database>::Row>,

    for<'e> i64: Encode<'e, DB> + Type<DB>,
    for<'e> Option<i64>: Encode<'e, DB> + Type<DB>,
    for<'e> &'e str: Encode<'e, DB> + Type<DB>,
{
    async fn get_ttl(
        &self,
        key: &str,
        ttl: Option<Duration>,
    ) -> Result<Option<String>, RepositoryError> {
        let now = Utc::now();

        let row: Option<KeyValueRow> =
            sqlx::query_as("SELECT expiration, value FROM key_value WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.db)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "Failed to get key-value registry: sqlx error");
                    error
                })?;

        if let Some(row) = row {
            if matches!(row.expiration, Some(expiration) if now.timestamp_millis() > expiration) {
                let _ = sqlx::query("DELETE FROM key_value WHERE key = $1")
                    .bind(key)
                    .execute(&self.db)
                    .await
                    .map_err(|error| {
                        tracing::error!(
                            %error,
                            "Failed to delete expired key-value registry: sqlx error",
                        );
                    });

                Ok(None)
            } else if let Some(ttl) = ttl {
                sqlx::query("UPDATE key_value SET expiration = $1 WHERE key = $2")
                    .bind((now + ttl).timestamp_millis())
                    .bind(key)
                    .execute(&self.db)
                    .await
                    .map_err(|error| {
                        tracing::error!(
                            %error,
                            "Failed to update ttl of key-value registry: sqlx error",
                        );
                        error
                    })?;

                Ok(Some(row.value))
            } else {
                Ok(Some(row.value))
            }
        } else {
            Ok(None)
        }
    }

    async fn set_ttl(
        &self,
        key: &str,
        value: &str,
        ttl: Option<Duration>,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();

        if self.get_ttl(key, None).await?.is_some() {
            sqlx::query(
                "UPDATE key_value \
                SET expiration = $1, value = $2 \
                WHERE key = $3",
            )
            .bind(ttl.map(|exp| (now + exp).timestamp_millis()))
            .bind(value)
            .bind(key)
            .execute(&self.db)
            .await
            .map(|_| ())
            .map_err(|error| {
                tracing::error!(%error, "Failed to update key-value registry: sqlx error");
                error.into()
            })
        } else {
            sqlx::query(
                "INSERT INTO key_value \
                (key, created_at, expiration, value) \
                VALUES ($1, $2, $3, $4)",
            )
            .bind(key)
            .bind(now.timestamp_millis())
            .bind(ttl.map(|exp| (now + exp).timestamp_millis()))
            .bind(value)
            .execute(&self.db)
            .await
            .map(|_| ())
            .map_err(|error| {
                tracing::error!(%error, "Failed to create key-value registry: sqlx error");
                error.into()
            })
        }
    }

    async fn delete(&self, key: &str) -> Result<Option<String>, RepositoryError> {
        let now = Utc::now();

        sqlx::query_as("DELETE FROM key_value WHERE key = $1 RETURNING expiration, value")
            .bind(key)
            .fetch_optional(&self.db)
            .await
            .map(|v: Option<KeyValueRow>| match v {
                Some(v) => {
                    let expired = v
                        .expiration
                        .map_or(false, |exp| now.timestamp_millis() > exp);

                    if expired {
                        None
                    } else {
                        Some(KeyValueRow::into_string(v))
                    }
                }
                None => None,
            })
            .map_err(|error| {
                tracing::error!(%error, "Failed to delete key-value registry: sqlx error");
                error.into()
            })
    }
}
