use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::dao::{MailDaoError, SentMail, SentMailDao};

fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    if let Ok(dt) =
        PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT)
    {
        return Ok(dt);
    }
    let sqlite_format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
    )
    .unwrap();
    if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
        return Ok(dt);
    }
    let sqlite_simple =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    PrimitiveDateTime::parse(s, &sqlite_simple)
}

fn format_datetime(dt: &PrimitiveDateTime) -> Result<String, MailDaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))
}

#[derive(Debug, sqlx::FromRow)]
struct SentMailDb {
    id: Vec<u8>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
    to_address: String,
    subject: String,
    body: String,
    status: String,
    error: Option<String>,
    sent_at: Option<String>,
}

impl TryFrom<&SentMailDb> for SentMail {
    type Error = MailDaoError;

    fn try_from(db: &SentMailDb) -> Result<Self, Self::Error> {
        Ok(SentMail {
            id: Uuid::from_slice(&db.id)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            deleted: db
                .deleted
                .as_ref()
                .map(|d| parse_datetime(d))
                .transpose()
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            version: Uuid::from_slice(&db.version)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            to_address: Arc::from(db.to_address.as_str()),
            subject: Arc::from(db.subject.as_str()),
            body: Arc::from(db.body.as_str()),
            status: Arc::from(db.status.as_str()),
            error: db.error.as_deref().map(Arc::from),
            sent_at: db
                .sent_at
                .as_ref()
                .map(|d| parse_datetime(d))
                .transpose()
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
        })
    }
}

pub struct SentMailDaoSqlite {
    pool: Arc<SqlitePool>,
}

impl SentMailDaoSqlite {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SentMailDao for SentMailDaoSqlite {
    async fn all(&self) -> Result<Arc<[SentMail]>, MailDaoError> {
        let rows = sqlx::query_as::<_, SentMailDb>(
            "SELECT id, created, deleted, version, to_address, subject, body, status, error, sent_at \
             FROM sent_mails ORDER BY created DESC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(SentMail::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(&self, mail: &SentMail) -> Result<(), MailDaoError> {
        let id = mail.id.as_bytes().to_vec();
        let version = mail.version.as_bytes().to_vec();
        let created = format_datetime(&mail.created)?;
        let sent_at = mail
            .sent_at
            .as_ref()
            .map(format_datetime)
            .transpose()?;

        sqlx::query(
            "INSERT INTO sent_mails (id, created, deleted, version, to_address, subject, body, status, error, sent_at) \
             VALUES (?, ?, NULL, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(created)
        .bind(version)
        .bind(mail.to_address.as_ref())
        .bind(mail.subject.as_ref())
        .bind(mail.body.as_ref())
        .bind(mail.status.as_ref())
        .bind(mail.error.as_deref())
        .bind(sent_at)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");
        sqlx::query(
            "CREATE TABLE sent_mails (
                id BLOB PRIMARY KEY,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL,
                to_address TEXT NOT NULL,
                subject TEXT NOT NULL,
                body TEXT NOT NULL,
                status TEXT NOT NULL,
                error TEXT,
                sent_at TEXT
            )",
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");
        Arc::new(pool)
    }

    fn sample_sent_mail() -> SentMail {
        SentMail {
            id: Uuid::new_v4(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
                time::Time::from_hms(10, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
            to_address: Arc::from("user@example.com"),
            subject: Arc::from("Test Subject"),
            body: Arc::from("Test Body"),
            status: Arc::from("sent"),
            error: None,
            sent_at: Some(time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
                time::Time::from_hms(10, 0, 1).unwrap(),
            )),
        }
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let pool = setup_db().await;
        let dao = SentMailDaoSqlite::new(pool);

        let mail = sample_sent_mail();
        dao.create(&mail).await.unwrap();

        let all = dao.all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].to_address.as_ref(), "user@example.com");
        assert_eq!(all[0].subject.as_ref(), "Test Subject");
        assert_eq!(all[0].status.as_ref(), "sent");
        assert!(all[0].error.is_none());
        assert!(all[0].sent_at.is_some());
    }

    #[tokio::test]
    async fn test_create_failed_mail() {
        let pool = setup_db().await;
        let dao = SentMailDaoSqlite::new(pool);

        let mail = SentMail {
            id: Uuid::new_v4(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::April, 3).unwrap(),
                time::Time::from_hms(10, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
            to_address: Arc::from("user@example.com"),
            subject: Arc::from("Test"),
            body: Arc::from("Body"),
            status: Arc::from("failed"),
            error: Some(Arc::from("Connection refused")),
            sent_at: None,
        };
        dao.create(&mail).await.unwrap();

        let all = dao.all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status.as_ref(), "failed");
        assert_eq!(all[0].error.as_deref(), Some("Connection refused"));
        assert!(all[0].sent_at.is_none());
    }

    #[tokio::test]
    async fn test_all_ordered_by_created_desc() {
        let pool = setup_db().await;
        let dao = SentMailDaoSqlite::new(pool);

        let mail1 = SentMail {
            id: Uuid::new_v4(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::April, 1).unwrap(),
                time::Time::from_hms(10, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
            to_address: Arc::from("first@example.com"),
            subject: Arc::from("First"),
            body: Arc::from("Body"),
            status: Arc::from("sent"),
            error: None,
            sent_at: None,
        };
        let mail2 = SentMail {
            id: Uuid::new_v4(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::April, 2).unwrap(),
                time::Time::from_hms(10, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
            to_address: Arc::from("second@example.com"),
            subject: Arc::from("Second"),
            body: Arc::from("Body"),
            status: Arc::from("sent"),
            error: None,
            sent_at: None,
        };

        dao.create(&mail1).await.unwrap();
        dao.create(&mail2).await.unwrap();

        let all = dao.all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].to_address.as_ref(), "second@example.com");
        assert_eq!(all[1].to_address.as_ref(), "first@example.com");
    }

    #[tokio::test]
    async fn test_all_empty() {
        let pool = setup_db().await;
        let dao = SentMailDaoSqlite::new(pool);

        let all = dao.all().await.unwrap();
        assert!(all.is_empty());
    }
}
