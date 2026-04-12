use async_trait::async_trait;
use genossi_dao::backup::{ActionBackupRow, BackupDao, DocumentBackupRow, MemberBackupRow};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct BackupDaoImpl {
    pool: Arc<SqlitePool>,
}

impl BackupDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MemberBackupDb {
    member_number: i64,
    salutation: Option<String>,
    title: Option<String>,
    first_name: String,
    last_name: String,
    company: Option<String>,
    street: Option<String>,
    house_number: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    email: Option<String>,
    bank_account: Option<String>,
    join_date: String,
    exit_date: Option<String>,
    shares_at_joining: i32,
    shares_at_date: i32,
    comment: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ActionBackupDb {
    member_number: i64,
    first_name: String,
    last_name: String,
    action_type: String,
    date: String,
    shares_change: i32,
    transfer_member_number: Option<i64>,
    effective_date: Option<String>,
    comment: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct DocumentBackupDb {
    member_number: i64,
    first_name: String,
    last_name: String,
    document_type: String,
    file_name: String,
    relative_path: String,
}

#[async_trait]
impl BackupDao for BackupDaoImpl {
    async fn members_at_date(&self, date: time::Date) -> Result<Arc<[MemberBackupRow]>, DaoError> {
        let format = time::format_description::parse("[year]-[month]-[day]").unwrap();
        let date_str = date
            .format(&format)
            .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;

        let rows = sqlx::query_as::<_, MemberBackupDb>(
            "SELECT m.member_number, m.salutation, m.title, m.first_name, m.last_name, \
                    m.company, m.street, m.house_number, m.postal_code, m.city, \
                    m.email, m.bank_account, m.join_date, m.exit_date, m.shares_at_joining, \
                    COALESCE(SUM( \
                        CASE WHEN a.date <= ? AND a.deleted IS NULL THEN a.shares_change ELSE 0 END \
                    ), 0) as shares_at_date, \
                    m.comment \
             FROM member m \
             LEFT JOIN member_action a ON m.id = a.member_id \
             WHERE m.deleted IS NULL \
               AND m.join_date <= ? \
               AND (m.exit_date IS NULL OR m.exit_date > ?) \
               AND (m.status IS NULL OR m.status != 'FehlerhaftErfasst') \
             GROUP BY m.id \
             ORDER BY m.member_number",
        )
        .bind(&date_str)
        .bind(&date_str)
        .bind(&date_str)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        let result: Vec<MemberBackupRow> = rows
            .into_iter()
            .map(|r| MemberBackupRow {
                member_number: r.member_number,
                salutation: r.salutation.map(|s| Arc::from(s.as_str())),
                title: r.title.map(|s| Arc::from(s.as_str())),
                first_name: Arc::from(r.first_name.as_str()),
                last_name: Arc::from(r.last_name.as_str()),
                company: r.company.map(|s| Arc::from(s.as_str())),
                street: r.street.map(|s| Arc::from(s.as_str())),
                house_number: r.house_number.map(|s| Arc::from(s.as_str())),
                postal_code: r.postal_code.map(|s| Arc::from(s.as_str())),
                city: r.city.map(|s| Arc::from(s.as_str())),
                email: r.email.map(|s| Arc::from(s.as_str())),
                bank_account: r.bank_account.map(|s| Arc::from(s.as_str())),
                join_date: Arc::from(r.join_date.as_str()),
                exit_date: r.exit_date.map(|s| Arc::from(s.as_str())),
                shares_at_joining: r.shares_at_joining,
                shares_at_date: r.shares_at_date,
                comment: r.comment.map(|s| Arc::from(s.as_str())),
            })
            .collect();

        Ok(result.into())
    }

    async fn all_actions(&self) -> Result<Arc<[ActionBackupRow]>, DaoError> {
        let rows = sqlx::query_as::<_, ActionBackupDb>(
            "SELECT m.member_number, m.first_name, m.last_name, \
                    a.action_type, a.date, a.shares_change, \
                    tm.member_number as transfer_member_number, \
                    a.effective_date, a.comment \
             FROM member_action a \
             INNER JOIN member m ON a.member_id = m.id \
             LEFT JOIN member tm ON a.transfer_member_id = tm.id \
             WHERE a.deleted IS NULL \
             ORDER BY m.member_number, a.date",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        let result: Vec<ActionBackupRow> = rows
            .into_iter()
            .map(|r| ActionBackupRow {
                member_number: r.member_number,
                first_name: Arc::from(r.first_name.as_str()),
                last_name: Arc::from(r.last_name.as_str()),
                action_type: Arc::from(r.action_type.as_str()),
                date: Arc::from(r.date.as_str()),
                shares_change: r.shares_change,
                transfer_member_number: r.transfer_member_number,
                effective_date: r.effective_date.map(|s| Arc::from(s.as_str())),
                comment: r.comment.map(|s| Arc::from(s.as_str())),
            })
            .collect();

        Ok(result.into())
    }

    async fn all_documents(&self) -> Result<Arc<[DocumentBackupRow]>, DaoError> {
        let rows = sqlx::query_as::<_, DocumentBackupDb>(
            "SELECT m.member_number, m.first_name, m.last_name, \
                    d.document_type, d.file_name, d.relative_path \
             FROM member_document d \
             INNER JOIN member m ON d.member_id = m.id \
             WHERE d.deleted IS NULL AND m.deleted IS NULL \
             ORDER BY m.member_number",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        let result: Vec<DocumentBackupRow> = rows
            .into_iter()
            .map(|r| DocumentBackupRow {
                member_number: r.member_number,
                first_name: Arc::from(r.first_name.as_str()),
                last_name: Arc::from(r.last_name.as_str()),
                document_type: Arc::from(r.document_type.as_str()),
                file_name: Arc::from(r.file_name.as_str()),
                relative_path: Arc::from(r.relative_path.as_str()),
            })
            .collect();

        Ok(result.into())
    }
}
