use async_trait::async_trait;
use genossi_dao::helper_token::{HelperTokenDao, HelperTokenEntity};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::assembly::parse_datetime;
use crate::TransactionImpl;

fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}

#[derive(Debug, sqlx::FromRow)]
struct HelperTokenDb {
    id: Vec<u8>,
    assembly_id: Vec<u8>,
    memo: String,
    token_hash: String,
    created: String,
    used_at: Option<String>,
    session_id: Option<String>,
    revoked_at: Option<String>,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&HelperTokenDb> for HelperTokenEntity {
    type Error = DaoError;

    fn try_from(db: &HelperTokenDb) -> Result<Self, Self::Error> {
        Ok(HelperTokenEntity {
            id: Uuid::from_slice(&db.id)?,
            assembly_id: Uuid::from_slice(&db.assembly_id)?,
            memo: Arc::from(db.memo.as_str()),
            token_hash: Arc::from(db.token_hash.as_str()),
            created: parse_datetime(&db.created)?,
            used_at: db
                .used_at
                .as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
            session_id: db.session_id.as_deref().map(Arc::from),
            revoked_at: db
                .revoked_at
                .as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
            deleted: db
                .deleted
                .as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RedeemRow {
    id: Vec<u8>,
    assembly_id: Vec<u8>,
}

#[derive(Debug, sqlx::FromRow)]
struct LookupStatusRow {
    used_at: Option<String>,
    revoked_at: Option<String>,
}

pub struct HelperTokenDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl HelperTokenDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HelperTokenDao for HelperTokenDaoImpl {
    type Transaction = TransactionImpl;

    async fn dump_all(
        &self,
        tx: Self::Transaction,
    ) -> Result<Arc<[HelperTokenEntity]>, DaoError> {
        let rows = sqlx::query_as::<_, HelperTokenDb>(
            "SELECT id, assembly_id, memo, token_hash, created, used_at, session_id, \
             revoked_at, deleted, version FROM helper_token ORDER BY created DESC",
        )
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(HelperTokenEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn create(
        &self,
        entity: &HelperTokenEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let assembly_id = entity.assembly_id.as_bytes().to_vec();
        let version = entity.version.as_bytes().to_vec();
        let memo = entity.memo.to_string();
        let token_hash = entity.token_hash.to_string();
        let created = format_dt(&entity.created)?;
        let used_at = entity.used_at.as_ref().map(format_dt).transpose()?;
        let session_id = entity.session_id.as_ref().map(|s| s.to_string());
        let revoked_at = entity.revoked_at.as_ref().map(format_dt).transpose()?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        sqlx::query(
            "INSERT INTO helper_token (id, assembly_id, memo, token_hash, created, \
             used_at, session_id, revoked_at, deleted, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(assembly_id)
        .bind(memo)
        .bind(token_hash)
        .bind(created)
        .bind(used_at)
        .bind(session_id)
        .bind(revoked_at)
        .bind(deleted)
        .bind(version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(())
    }

    async fn update(
        &self,
        entity: &HelperTokenEntity,
        _process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = entity.id.as_bytes().to_vec();
        let old_version = entity.version.as_bytes().to_vec();
        let new_version = Uuid::new_v4().as_bytes().to_vec();
        let assembly_id = entity.assembly_id.as_bytes().to_vec();
        let memo = entity.memo.to_string();
        let token_hash = entity.token_hash.to_string();
        let used_at = entity.used_at.as_ref().map(format_dt).transpose()?;
        let session_id = entity.session_id.as_ref().map(|s| s.to_string());
        let revoked_at = entity.revoked_at.as_ref().map(format_dt).transpose()?;
        let deleted = entity.deleted.as_ref().map(format_dt).transpose()?;

        // Pre-condition: row must exist and not be soft-deleted. Without this
        // check, a missing-id and a version mismatch would both surface as
        // ConflictError, which conflates two distinct error semantics.
        let exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM helper_token WHERE id = ? AND deleted IS NULL",
        )
        .bind(id.clone())
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if exists == 0 {
            return Err(DaoError::NotFound);
        }

        let rows_affected = sqlx::query(
            "UPDATE helper_token SET assembly_id = ?, memo = ?, token_hash = ?, \
             used_at = ?, session_id = ?, revoked_at = ?, deleted = ?, version = ? \
             WHERE id = ? AND version = ? AND deleted IS NULL",
        )
        .bind(assembly_id)
        .bind(memo)
        .bind(token_hash)
        .bind(used_at)
        .bind(session_id)
        .bind(revoked_at)
        .bind(deleted)
        .bind(new_version)
        .bind(id)
        .bind(old_version)
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
        }

        Ok(())
    }

    async fn atomic_redeem(
        &self,
        token_hash: &str,
        used_at: PrimitiveDateTime,
        tx: Self::Transaction,
    ) -> Result<Option<(Uuid, Uuid)>, DaoError> {
        // RESEARCH §Pattern 1 — VERBATIM. Use query_as::<_, RedeemRow> +
        // fetch_optional (NOT query_as! macro — Pitfall 1: SQLx 0.8
        // RETURNING-nullability bug; NOT fetch_one — 0-row case is valid and
        // must not be a database error).
        let used_at_str = format_dt(&used_at)?;
        let row: Option<RedeemRow> = sqlx::query_as::<_, RedeemRow>(
            "UPDATE helper_token \
             SET used_at = ? \
             WHERE token_hash = ? \
               AND used_at IS NULL \
               AND revoked_at IS NULL \
               AND deleted IS NULL \
             RETURNING id, assembly_id",
        )
        .bind(used_at_str)
        .bind(token_hash)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        Ok(match row {
            Some(r) => Some((Uuid::from_slice(&r.id)?, Uuid::from_slice(&r.assembly_id)?)),
            None => None,
        })
    }

    async fn set_session_id(
        &self,
        token_id: Uuid,
        session_id: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        let id = token_id.as_bytes().to_vec();
        let result = sqlx::query("UPDATE helper_token SET session_id = ? WHERE id = ?")
            .bind(session_id)
            .bind(id)
            .execute(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        if result.rows_affected() == 0 {
            return Err(DaoError::NotFound);
        }
        Ok(())
    }

    async fn lookup_status(
        &self,
        token_hash: &str,
        tx: Self::Transaction,
    ) -> Result<
        Option<(
            Option<PrimitiveDateTime>,
            Option<PrimitiveDateTime>,
        )>,
        DaoError,
    > {
        let row: Option<LookupStatusRow> = sqlx::query_as::<_, LookupStatusRow>(
            "SELECT used_at, revoked_at FROM helper_token \
             WHERE token_hash = ? AND deleted IS NULL",
        )
        .bind(token_hash)
        .fetch_optional(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        match row {
            None => Ok(None),
            Some(r) => Ok(Some((
                r.used_at.as_ref().map(|s| parse_datetime(s)).transpose()?,
                r.revoked_at
                    .as_ref()
                    .map(|s| parse_datetime(s))
                    .transpose()?,
            ))),
        }
    }

    async fn all_for_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Arc<[HelperTokenEntity]>, DaoError> {
        let aid = assembly_id.as_bytes().to_vec();
        let rows = sqlx::query_as::<_, HelperTokenDb>(
            "SELECT id, assembly_id, memo, token_hash, created, used_at, session_id, \
             revoked_at, deleted, version FROM helper_token \
             WHERE assembly_id = ? AND deleted IS NULL ORDER BY created DESC",
        )
        .bind(aid)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        rows.iter()
            .map(HelperTokenEntity::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into())
    }

    async fn list_session_ids_for_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Vec<Arc<str>>, DaoError> {
        // D-12 (Phase 3): Cascade-Discovery via session_id-FK.
        // Caller (AssemblyServiceImpl::close_assembly, Plan 05) iterates the
        // result and calls PermissionDao::delete_session for each id.
        // Filters: assembly_id parameterized via bind (T-03-02-02 mitigation),
        // session_id IS NOT NULL excludes revoked/never-redeemed tokens,
        // deleted IS NULL excludes soft-deleted token rows.
        let aid = assembly_id.as_bytes().to_vec();
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT session_id FROM helper_token \
             WHERE assembly_id = ? AND session_id IS NOT NULL AND deleted IS NULL",
        )
        .bind(aid)
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(rows.into_iter().map(|s| Arc::from(s.as_str())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionDaoImpl;
    use genossi_dao::{Transaction, TransactionDao};

    /// Bring up an in-memory SQLite pool with the assembly + helper_token schema
    /// applied. We don't run the full migration set here because that would
    /// require the entire Member/Application/etc. graph; we only need the
    /// FK-target (assembly) and the table under test.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        // FK enforcement is off by default in SQLite — turn it on so the
        // ON DELETE RESTRICT/SET NULL semantics are exercised.
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable FKs");

        sqlx::query(
            "CREATE TABLE assembly (
                id BLOB PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                date TEXT NOT NULL,
                location TEXT,
                status TEXT NOT NULL DEFAULT 'Preparation',
                opened_at TEXT,
                closed_at TEXT,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create assembly table");

        sqlx::query(
            "CREATE TABLE helper_token (
                id BLOB PRIMARY KEY NOT NULL,
                assembly_id BLOB NOT NULL,
                memo TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                created TEXT NOT NULL,
                used_at TEXT,
                session_id TEXT,
                revoked_at TEXT,
                deleted TEXT,
                version BLOB NOT NULL,
                FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT
            )",
        )
        .execute(&pool)
        .await
        .expect("create helper_token table");

        sqlx::query("CREATE UNIQUE INDEX idx_helper_token_token_hash ON helper_token(token_hash)")
            .execute(&pool)
            .await
            .expect("create unique index");

        Arc::new(pool)
    }

    /// Insert a minimal Assembly row directly via SQL (FK target). We bypass
    /// the AssemblyDao here to keep this unit-test scope small and focused on
    /// helper_token semantics.
    async fn create_assembly_for_test(pool: &SqlitePool, id: Uuid) {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let date_str = format_dt(&datetime).unwrap();
        sqlx::query(
            "INSERT INTO assembly (id, name, date, status, created, version) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().to_vec())
        .bind("GV Test")
        .bind(date_str.clone())
        .bind("Open")
        .bind(date_str)
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .execute(pool)
        .await
        .expect("insert assembly");
    }

    fn make_token(assembly_id: Uuid, token_hash: &str) -> HelperTokenEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 3).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        HelperTokenEntity {
            id: Uuid::new_v4(),
            assembly_id,
            memo: Arc::from("Anna"),
            token_hash: Arc::from(token_hash),
            created: datetime,
            used_at: None,
            session_id: None,
            revoked_at: None,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn now_pdt() -> PrimitiveDateTime {
        let now = time::OffsetDateTime::now_utc();
        PrimitiveDateTime::new(now.date(), now.time())
    }

    #[tokio::test]
    async fn test_create_and_find_helper_token() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let token = make_token(assembly_id, "hash_create");
        let token_id = token.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&token, "test", tx.clone()).await.unwrap();

        let found = dao.find_by_id(token_id, tx.clone()).await.unwrap();
        let found = found.expect("entity must be found");
        assert_eq!(found.id, token_id);
        assert_eq!(found.assembly_id, assembly_id);
        assert_eq!(found.memo.as_ref(), "Anna");
        assert_eq!(found.token_hash.as_ref(), "hash_create");
        assert!(found.used_at.is_none());
        assert!(found.revoked_at.is_none());
        assert!(found.session_id.is_none());

        let listing = dao
            .all_for_assembly(assembly_id, tx.clone())
            .await
            .unwrap();
        assert_eq!(listing.len(), 1);
        assert_eq!(listing[0].id, token_id);

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_atomic_redeem_first_call_succeeds() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let token = make_token(assembly_id, "hash1");
        let token_id = token.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&token, "test", tx.clone()).await.unwrap();

        // First redeem: succeeds, returns (token_id, assembly_id).
        let result = dao
            .atomic_redeem("hash1", now_pdt(), tx.clone())
            .await
            .unwrap();
        assert_eq!(result, Some((token_id, assembly_id)));

        // Second redeem on the same hash: 0 rows affected → None
        // (HLPR-04 race-safety on DAO level).
        let result2 = dao
            .atomic_redeem("hash1", now_pdt(), tx.clone())
            .await
            .unwrap();
        assert_eq!(result2, None);

        // After the redeem, lookup_status reports used_at=Some, revoked_at=None.
        let status = dao.lookup_status("hash1", tx.clone()).await.unwrap();
        let (used_at, revoked_at) = status.expect("token row exists");
        assert!(used_at.is_some(), "used_at must be set after redeem");
        assert!(revoked_at.is_none(), "revoked_at must remain None");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_atomic_redeem_revoked_token_returns_none() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        // Create a token already marked revoked (Phase-2 service will set
        // revoked_at via update; here we shortcut for the unit test).
        let mut token = make_token(assembly_id, "hash_revoked");
        token.revoked_at = Some(now_pdt());

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&token, "test", tx.clone()).await.unwrap();

        // Redeem must NOT succeed.
        let result = dao
            .atomic_redeem("hash_revoked", now_pdt(), tx.clone())
            .await
            .unwrap();
        assert_eq!(result, None);

        // lookup_status discriminates: row exists, revoked_at is Some, used_at
        // is None → REST-Layer maps to 403 (D-24).
        let status = dao.lookup_status("hash_revoked", tx.clone()).await.unwrap();
        let (used_at, revoked_at) = status.expect("token row exists");
        assert!(used_at.is_none());
        assert!(revoked_at.is_some());

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_atomic_redeem_unknown_hash_returns_none() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao
            .atomic_redeem("never_existed", now_pdt(), tx.clone())
            .await
            .unwrap();
        assert_eq!(result, None);

        // lookup_status returns None → REST-Layer maps to 404 (D-24).
        let status = dao
            .lookup_status("never_existed", tx.clone())
            .await
            .unwrap();
        assert!(status.is_none(), "unknown hash must yield None");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_set_session_id_updates_column() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let token = make_token(assembly_id, "hash_session");
        let token_id = token.id;

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&token, "test", tx.clone()).await.unwrap();

        dao.set_session_id(token_id, "session-xyz", tx.clone())
            .await
            .unwrap();

        let found = dao.find_by_id(token_id, tx.clone()).await.unwrap().unwrap();
        assert_eq!(
            found.session_id.as_deref(),
            Some("session-xyz"),
            "session_id must be persisted by set_session_id"
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_set_session_id_unknown_id_returns_not_found() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();
        let result = dao
            .set_session_id(Uuid::new_v4(), "session-xyz", tx.clone())
            .await;
        assert!(
            matches!(result, Err(DaoError::NotFound)),
            "expected NotFound, got: {:?}",
            result
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_update_with_version_mismatch_returns_conflict() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let token = make_token(assembly_id, "hash_conflict");

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&token, "test", tx.clone()).await.unwrap();

        // Build an update with a stale version (random UUID, not the persisted one).
        let mut stale = token.clone();
        stale.version = Uuid::new_v4();
        stale.memo = Arc::from("Bernd");

        let result = dao.update(&stale, "test", tx.clone()).await;
        assert!(
            matches!(result, Err(DaoError::ConflictError(_))),
            "expected ConflictError, got: {:?}",
            result
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_all_for_assembly_filters_deleted() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        // Active token
        let active = make_token(assembly_id, "hash_active");
        // Soft-deleted token
        let mut deleted = make_token(assembly_id, "hash_deleted");
        deleted.deleted = Some(now_pdt());

        let tx = tx_dao.transaction().await.unwrap();
        dao.create(&active, "test", tx.clone()).await.unwrap();
        dao.create(&deleted, "test", tx.clone()).await.unwrap();

        let listing = dao
            .all_for_assembly(assembly_id, tx.clone())
            .await
            .unwrap();
        assert_eq!(listing.len(), 1, "soft-deleted tokens must be filtered");
        assert_eq!(listing[0].id, active.id);

        tx.commit().await.unwrap();
    }

    // ---------------------------------------------------------------
    // Phase 3 Plan 02: list_session_ids_for_assembly (D-12)
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_list_session_ids_for_assembly_returns_redeemed_only() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();

        // Token A: redeemed (used_at + session_id set)
        let mut token_a = make_token(assembly_id, "tok-a-hash");
        token_a.used_at = Some(now_pdt());
        token_a.session_id = Some(Arc::from("sess-A"));
        dao.create(&token_a, "test", tx.clone()).await.unwrap();

        // Token B: open (no session_id, no used_at)
        let token_b = make_token(assembly_id, "tok-b-hash");
        dao.create(&token_b, "test", tx.clone()).await.unwrap();

        // Token C: revoked AND soft-deleted (deleted IS NOT NULL)
        // — even if it had a session_id, it must be excluded.
        let mut token_c = make_token(assembly_id, "tok-c-hash");
        token_c.session_id = Some(Arc::from("sess-C"));
        token_c.revoked_at = Some(now_pdt());
        token_c.deleted = Some(now_pdt());
        dao.create(&token_c, "test", tx.clone()).await.unwrap();

        let result = dao
            .list_session_ids_for_assembly(assembly_id, tx.clone())
            .await
            .unwrap();

        assert_eq!(
            result.len(),
            1,
            "exactly 1 redeemed-and-active session_id must be returned"
        );
        assert_eq!(result[0].as_ref(), "sess-A");

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_list_session_ids_for_assembly_empty_for_unknown_assembly() {
        let pool = setup_db().await;
        let assembly_id = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_id).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();

        // Random UUID — no FK target, no rows. The DAO method just runs a
        // SELECT, so it returns an empty Vec regardless of FK existence.
        let unknown_assembly = Uuid::new_v4();
        let result = dao
            .list_session_ids_for_assembly(unknown_assembly, tx.clone())
            .await
            .unwrap();
        assert!(
            result.is_empty(),
            "no rows for an unknown assembly_id, got {:?}",
            result
        );

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_list_session_ids_for_assembly_excludes_other_assemblies() {
        let pool = setup_db().await;
        let assembly_a = Uuid::new_v4();
        let assembly_b = Uuid::new_v4();
        create_assembly_for_test(&pool, assembly_a).await;
        create_assembly_for_test(&pool, assembly_b).await;

        let dao = HelperTokenDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();

        // Assembly A: 1 redeemed token with session "sess-AAA"
        let mut tok_a = make_token(assembly_a, "hash-aaa");
        tok_a.used_at = Some(now_pdt());
        tok_a.session_id = Some(Arc::from("sess-AAA"));
        dao.create(&tok_a, "test", tx.clone()).await.unwrap();

        // Assembly B: 1 redeemed token with session "sess-BBB"
        let mut tok_b = make_token(assembly_b, "hash-bbb");
        tok_b.used_at = Some(now_pdt());
        tok_b.session_id = Some(Arc::from("sess-BBB"));
        dao.create(&tok_b, "test", tx.clone()).await.unwrap();

        // Query Assembly A → only sess-AAA, never sess-BBB.
        let result = dao
            .list_session_ids_for_assembly(assembly_a, tx.clone())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_ref(), "sess-AAA");

        // Query Assembly B → only sess-BBB.
        let result_b = dao
            .list_session_ids_for_assembly(assembly_b, tx.clone())
            .await
            .unwrap();
        assert_eq!(result_b.len(), 1);
        assert_eq!(result_b[0].as_ref(), "sess-BBB");

        tx.commit().await.unwrap();
    }
}
