use super::RepositoryError;
use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use sqlx::{
    encode::IsNull, error::BoxDynError, ColumnIndex, Database, Decode, Encode, Executor, FromRow,
    IntoArguments, Pool, Row, Type,
};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct IpBanData {
    pub ip: IpAddr,
    pub created_at: DateTime<Utc>,
    pub expiration: Option<DateTime<Utc>>,
    pub reason: Option<String>,
}

pub trait IpBansRepository: Clone + Send + Sync {
    fn add_ban(
        &self,
        ip: IpAddr,
        duration: Option<Duration>,
        reason: Option<String>,
    ) -> impl Future<Output = Result<IpBanData, RepositoryError>> + Send;

    fn is_banned(
        &self,
        ip: IpAddr,
    ) -> impl Future<Output = Result<Option<IpBanData>, RepositoryError>> + Send;

    fn remove_ban(
        &self,
        ip: IpAddr,
    ) -> impl Future<Output = Result<Option<IpBanData>, RepositoryError>> + Send;

    fn get_bans(&self) -> impl Future<Output = Result<Vec<IpBanData>, RepositoryError>> + Send;
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
struct IpBinaryData(IpAddr);

impl<DB: Database> Type<DB> for IpBinaryData
where
    Vec<u8>: Type<DB>,
{
    #[inline]
    fn type_info() -> DB::TypeInfo {
        <Vec<u8> as Type<DB>>::type_info()
    }
}

impl<'e, DB: Database> Encode<'e, DB> for IpBinaryData
where
    Vec<u8>: Encode<'e, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'e>,
    ) -> Result<IsNull, BoxDynError> {
        let mut vec = Vec::new();

        match self.0 {
            IpAddr::V4(ip) => {
                vec.push(4_u8);
                vec.extend(ip.octets());
            }
            IpAddr::V6(ip) => {
                vec.push(6_u8);
                vec.extend(ip.octets());
            }
        }

        vec.encode(buf)
    }
}

impl<'de, DB: Database> Decode<'de, DB> for IpBinaryData
where
    Vec<u8>: Decode<'de, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'de>) -> Result<Self, BoxDynError> {
        let value = Vec::<u8>::decode(value)?;

        let kind = *value.get(0).ok_or("Unexpected value IP size")?;

        if value.len() == 5 && kind == 4 {
            let ip = Ipv4Addr::new(value[1], value[2], value[3], value[4]);

            Ok(IpBinaryData(IpAddr::V4(ip)))
        } else if value.len() == 17 && kind == 6 {
            let ip = Ipv6Addr::from([
                value[1], value[2], value[3], value[4], value[5], value[6], value[7], value[8],
                value[9], value[10], value[11], value[12], value[13], value[14], value[15],
                value[16],
            ]);

            Ok(IpBinaryData(IpAddr::V6(ip)))
        } else {
            Err("Unexpected value IP type".into())
        }
    }
}

struct IpBanRow {
    ip: IpBinaryData,
    created_at: DateTime<Utc>,
    expiration: Option<DateTime<Utc>>,
    reason: Option<String>,
}

impl<'r, R: Row> FromRow<'r, R> for IpBanRow
where
    &'static str: ColumnIndex<R>,
    IpBinaryData: Decode<'r, R::Database> + Type<R::Database>,
    DateTime<Utc>: Decode<'r, R::Database> + Type<R::Database>,
    String: Decode<'r, R::Database> + Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        let data = IpBanRow {
            ip: row.try_get("ip")?,
            created_at: row.try_get("created_at")?,
            expiration: row.try_get("expiration")?,
            reason: row.try_get("reason")?,
        };

        Ok(data)
    }
}

impl IpBanData {
    #[inline]
    fn from_row(row: IpBanRow) -> Self {
        Self {
            ip: row.ip.0,
            created_at: row.created_at,
            expiration: row.expiration,
            reason: row.reason,
        }
    }
}

pub struct SqlxIpBansRepository<DB: Database> {
    db: Pool<DB>,
}

impl<DB: Database> Clone for SqlxIpBansRepository<DB> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

impl<DB: Database> SqlxIpBansRepository<DB> {
    #[inline]
    pub fn new(db: Pool<DB>) -> Self {
        Self { db }
    }
}

impl<DB> IpBansRepository for SqlxIpBansRepository<DB>
where
    DB: Database,
    for<'a> <DB as sqlx::Database>::Arguments<'a>: IntoArguments<'a, DB>,
    for<'a> &'a Pool<DB>: Executor<'a, Database = DB>,

    for<'r> IpBanRow: FromRow<'r, DB::Row>,

    for<'e> DateTime<Utc>: Encode<'e, DB> + Type<DB>,
    for<'e> Option<DateTime<Utc>>: Encode<'e, DB> + Type<DB>,
    for<'e> Option<String>: Encode<'e, DB> + Type<DB>,
    for<'e> IpBinaryData: Encode<'e, DB> + Type<DB>,
{
    async fn add_ban(
        &self,
        ip: IpAddr,
        duration: Option<Duration>,
        reason: Option<String>,
    ) -> Result<IpBanData, RepositoryError> {
        let now = Utc::now();
        let exp = duration.map(|exp| now + exp);

        if let Some(data) = self.is_banned(ip).await? {
            if exp != data.expiration || data.reason != reason {
                let row = sqlx::query_as(
                    "UPDATE ip_bans \
                    SET expiration = $1, reason = $2 \
                    WHERE ip = $3 \
                    RETURNING*",
                )
                .bind(exp)
                .bind(reason)
                .bind(IpBinaryData(ip))
                .fetch_one(&self.db)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "Failed to update IP ban registry: sqlx error");
                    error
                })?;

                Ok(IpBanData::from_row(row))
            } else {
                Ok(data)
            }
        } else {
            let row = sqlx::query_as(
                "INSERT INTO ip_bans \
                (ip, created_at, expiration, reason) \
                VALUES ($1, $2, $3, $4) \
                RETURNING *",
            )
            .bind(IpBinaryData(ip))
            .bind(now)
            .bind(duration.map(|exp| now + exp))
            .bind(reason)
            .fetch_one(&self.db)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to create IP ban registry: sqlx error");
                error
            })?;

            Ok(IpBanData::from_row(row))
        }
    }

    async fn is_banned(&self, ip: IpAddr) -> Result<Option<IpBanData>, RepositoryError> {
        let ip = IpBinaryData(ip);

        let row: Option<IpBanRow> = sqlx::query_as("SELECT * FROM ip_bans WHERE ip = $1")
            .bind(ip)
            .fetch_optional(&self.db)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to get IP ban registry: sqlx error");
                error
            })?;

        if let Some(row) = row {
            if matches!(row.expiration, Some(expiration) if Utc::now() > expiration) {
                let _ = sqlx::query("DELETE FROM ip_bans WHERE ip = $1")
                    .bind(ip)
                    .execute(&self.db)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Failed to delete expired IP ban registry: sqlx error");
                    });

                Ok(None)
            } else {
                Ok(Some(IpBanData::from_row(row)))
            }
        } else {
            Ok(None)
        }
    }

    async fn remove_ban(&self, ip: IpAddr) -> Result<Option<IpBanData>, RepositoryError> {
        sqlx::query_as("DELETE FROM ip_bans WHERE ip = $1 RETURNING *")
            .bind(IpBinaryData(ip))
            .fetch_optional(&self.db)
            .await
            .map(|v| v.map(IpBanData::from_row))
            .map_err(|error| {
                tracing::error!(%error, "Failed to delete IP ban registry: sqlx error");
                error.into()
            })
    }

    async fn get_bans(&self) -> Result<Vec<IpBanData>, RepositoryError> {
        sqlx::query_as("SELECT * FROM ip_bans")
            .fetch(&self.db)
            .try_filter_map(|v| async move { Ok(Some(IpBanData::from_row(v))) })
            .try_collect()
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to get all IP ban registries: sqlx error");
                error.into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{IpBansRepository, SqlxIpBansRepository};
    use chrono::Utc;
    use sqlx::{migrate, Sqlite, SqlitePool};
    use std::{
        collections::HashSet,
        net::{IpAddr, Ipv4Addr, Ipv6Addr},
        time::Duration,
    };
    use tokio::time::sleep;
    use uuid::Uuid;

    async fn get_repository() -> SqlxIpBansRepository<Sqlite> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        migrate!().run(&pool).await.unwrap();

        SqlxIpBansRepository::new(pool)
    }

    fn rand_ip() -> IpAddr {
        if rand::random::<bool>() {
            IpAddr::V6(Ipv6Addr::new(
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
            ))
        } else {
            IpAddr::V4(Ipv4Addr::new(
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
            ))
        }
    }

    #[tokio::test]
    async fn test_add_ban() {
        let repo = get_repository().await;

        let ip = rand_ip();

        let reason = Uuid::new_v4().to_string();

        let now = Utc::now();
        repo.add_ban(ip, None, Some(reason.clone())).await.unwrap();

        let ban = repo
            .is_banned(ip)
            .await
            .unwrap()
            .expect("The added ban was not registrered properly");

        assert_eq!(ban.ip, ip);
        assert_eq!(ban.reason.unwrap(), reason);
        assert_eq!(ban.created_at.timestamp(), now.timestamp());
    }

    #[tokio::test]
    async fn test_remove_ban() {
        let repo = get_repository().await;

        let ip = rand_ip();

        let result = repo.remove_ban(ip).await.unwrap();
        assert!(matches!(result, None));

        repo.add_ban(ip, None, None).await.unwrap();

        let result = repo.remove_ban(ip).await.unwrap();
        assert!(matches!(result, Some(_)));

        let result = repo.remove_ban(ip).await.unwrap();
        assert!(matches!(result, None));

        let result = repo.is_banned(ip).await.unwrap();
        assert!(matches!(result, None));
    }

    #[tokio::test]
    async fn test_ban_expiration() {
        let repo = get_repository().await;

        let ip = rand_ip();

        repo.add_ban(ip, Some(Duration::from_millis(100)), None)
            .await
            .unwrap();

        let result = repo.is_banned(ip).await.unwrap();
        assert!(matches!(result, Some(_)));

        sleep(Duration::from_millis(200)).await;
        let result = repo.is_banned(ip).await.unwrap();
        assert!(matches!(result, None));
    }

    #[tokio::test]
    async fn test_get_all_bans() {
        let repo = get_repository().await;

        let mut all_adds = HashSet::new();

        for _ in 0..10 {
            let ip = rand_ip();
            all_adds.insert(ip);

            repo.add_ban(ip, None, None).await.unwrap();
        }

        for data in repo.get_bans().await.unwrap() {
            assert!(all_adds.remove(&data.ip));
        }

        assert_eq!(all_adds.len(), 0);
    }
}
