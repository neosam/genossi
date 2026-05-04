use async_trait::async_trait;
use genossi_dao::attendance::{AttendanceDao, AttendanceMemberRow};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::TransactionImpl;

/// Module-local datetime formatter -- mirrors the analog helper in
/// `helper_token.rs`. Kept module-local because the call sites here all
/// touch SQLite TEXT-encoded datetimes, and the helper is short enough
/// that re-export would add coupling for no benefit.
fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}

/// Internal row type for `list_members_for_assembly`. Mirrors the
/// 7-column SELECT-whitelist (D-24, ATTN-01) -- adding a column here
/// requires a corresponding SELECT-list change AND a doc update on the
/// trait method to document the leak surface.
#[derive(Debug, sqlx::FromRow)]
struct AttendanceMemberRowDb {
    id: Vec<u8>,
    member_number: i64,
    first_name: String,
    last_name: String,
    salutation: Option<String>,
    title: Option<String>,
    is_present: i64,
}

/// SQLite implementation of `AttendanceDao`.
///
/// WR-03 caveat: the migration declares
/// `FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT`
/// and `FOREIGN KEY (member_id) REFERENCES member(id) ON DELETE RESTRICT`,
/// but the SqlitePool used by genossi_bin does not run
/// `PRAGMA foreign_keys=ON`. Referential integrity is enforced at the
/// service layer via `is_in_snapshot` (D-27).
pub struct AttendanceDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl AttendanceDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttendanceDao for AttendanceDaoImpl {
    type Transaction = TransactionImpl;

    async fn upsert_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        marked_at: PrimitiveDateTime,
        marked_by_user_id: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        // D-05: atomic toggle-on via SQLite UPSERT. Single statement,
        // race-free. Five-times invocation -> exactly one row.
        sqlx::query(
            "INSERT INTO attendance (assembly_id, member_id, marked_at, marked_by_user_id, deleted) \
             VALUES (?, ?, ?, ?, NULL) \
             ON CONFLICT(assembly_id, member_id) DO UPDATE SET \
                marked_at = excluded.marked_at, \
                marked_by_user_id = excluded.marked_by_user_id, \
                deleted = NULL",
        )
        .bind(assembly_id.as_bytes().to_vec())
        .bind(member_id.as_bytes().to_vec())
        .bind(format_dt(&marked_at)?)
        .bind(marked_by_user_id.to_string())
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }

    async fn soft_delete(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        deleted_at: PrimitiveDateTime,
        tx: Self::Transaction,
    ) -> Result<(), DaoError> {
        // D-06: idempotent toggle-off. rows_affected is intentionally
        // IGNORED (0 = never marked OR already absent -> No-Op,
        // 1 = was present, now soft-deleted).
        sqlx::query("UPDATE attendance SET deleted = ? WHERE assembly_id = ? AND member_id = ?")
            .bind(format_dt(&deleted_at)?)
            .bind(assembly_id.as_bytes().to_vec())
            .bind(member_id.as_bytes().to_vec())
            .execute(tx.tx.lock().await.as_mut())
            .await
            .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }

    async fn list_members_for_assembly(
        &self,
        assembly_id: Uuid,
        search: Option<String>,
        tx: Self::Transaction,
    ) -> Result<Arc<[AttendanceMemberRow]>, DaoError> {
        // D-25: substring search executed in DAO via SQL LIKE.
        // Pattern is `%trim(input)%`; LIKE-special-chars (%/_) inside
        // search input are functionally irrelevant for this UX (and
        // remain safely parameterised -- T-03-01-02).
        //
        // SELECT-whitelist of exactly 7 columns: PII-leak guard (D-24,
        // ATTN-01). Never `SELECT m.*` -- a future MemberEntity column
        // (e.g. new PII field) MUST NOT be reachable through this path.
        let aid = assembly_id.as_bytes().to_vec();
        let pattern: Option<String> = search.map(|s| format!("%{}%", s.trim()));
        let rows = sqlx::query_as::<_, AttendanceMemberRowDb>(
            "SELECT \
                m.id, m.member_number, m.first_name, m.last_name, \
                m.salutation, m.title, \
                CASE WHEN a.assembly_id IS NOT NULL AND a.deleted IS NULL THEN 1 ELSE 0 END AS is_present \
             FROM assembly_member_snapshot s \
             JOIN member m ON m.id = s.member_id AND m.deleted IS NULL \
             LEFT JOIN attendance a \
                 ON a.assembly_id = s.assembly_id AND a.member_id = m.id \
             WHERE s.assembly_id = ? \
               AND ( ? IS NULL \
                     OR (m.last_name || ' ' || m.first_name) LIKE ? COLLATE NOCASE \
                     OR CAST(m.member_number AS TEXT) LIKE ? \
                   ) \
             ORDER BY m.last_name COLLATE NOCASE, m.first_name COLLATE NOCASE",
        )
        .bind(aid)
        .bind(pattern.as_deref())
        .bind(pattern.as_deref())
        .bind(pattern.as_deref())
        .fetch_all(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

        let result: Vec<AttendanceMemberRow> = rows
            .into_iter()
            .map(|r| {
                Ok::<AttendanceMemberRow, DaoError>(AttendanceMemberRow {
                    member_id: Uuid::from_slice(&r.id)?,
                    member_number: r.member_number,
                    first_name: Arc::from(r.first_name.as_str()),
                    last_name: Arc::from(r.last_name.as_str()),
                    salutation: r.salutation.map(|s| Arc::from(s.as_str())),
                    title: r.title.map(|s| Arc::from(s.as_str())),
                    is_present: r.is_present != 0,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Arc::from(result))
    }

    async fn count_present_by_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError> {
        let aid = assembly_id.as_bytes().to_vec();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM attendance WHERE assembly_id = ? AND deleted IS NULL",
        )
        .bind(aid)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(count as u64)
    }

    async fn is_in_snapshot(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<bool, DaoError> {
        let aid = assembly_id.as_bytes().to_vec();
        let mid = member_id.as_bytes().to_vec();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM assembly_member_snapshot \
             WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(aid)
        .bind(mid)
        .fetch_one(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransactionDaoImpl;
    use genossi_dao::{Transaction, TransactionDao};

    /// In-memory schema bootstrap. We hand-roll the assembly + member +
    /// assembly_member_snapshot + attendance tables -- running the full
    /// migration set would require pulling in the entire workspace
    /// schema graph for unit tests. Plan-06 E2E tests exercise the real
    /// migration flow.
    async fn setup_db() -> Arc<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

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
            "CREATE TABLE member (
                id BLOB PRIMARY KEY NOT NULL,
                member_number INTEGER NOT NULL,
                first_name TEXT NOT NULL,
                last_name TEXT NOT NULL,
                salutation TEXT,
                title TEXT,
                created TEXT NOT NULL,
                deleted TEXT,
                version BLOB NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create member table");

        sqlx::query(
            "CREATE TABLE assembly_member_snapshot (
                assembly_id BLOB NOT NULL,
                member_id BLOB NOT NULL,
                captured_at TEXT NOT NULL,
                PRIMARY KEY (assembly_id, member_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create snapshot table");

        sqlx::query(
            "CREATE TABLE attendance (
                assembly_id BLOB NOT NULL,
                member_id BLOB NOT NULL,
                marked_at TEXT NOT NULL,
                marked_by_user_id TEXT NOT NULL,
                deleted TEXT,
                PRIMARY KEY (assembly_id, member_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create attendance table");

        Arc::new(pool)
    }

    fn now_pdt() -> PrimitiveDateTime {
        let now = time::OffsetDateTime::now_utc();
        PrimitiveDateTime::new(now.date(), now.time())
    }

    async fn insert_assembly(pool: &SqlitePool, id: Uuid) {
        let now = format_dt(&now_pdt()).unwrap();
        sqlx::query(
            "INSERT INTO assembly (id, name, date, status, created, version) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().to_vec())
        .bind("GV Test")
        .bind(now.clone())
        .bind("Open")
        .bind(now)
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .execute(pool)
        .await
        .expect("insert assembly");
    }

    async fn insert_member(
        pool: &SqlitePool,
        id: Uuid,
        member_number: i64,
        first: &str,
        last: &str,
    ) {
        let now = format_dt(&now_pdt()).unwrap();
        sqlx::query(
            "INSERT INTO member \
             (id, member_number, first_name, last_name, created, version) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().to_vec())
        .bind(member_number)
        .bind(first.to_string())
        .bind(last.to_string())
        .bind(now)
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .execute(pool)
        .await
        .expect("insert member");
    }

    async fn insert_snapshot(pool: &SqlitePool, assembly_id: Uuid, member_id: Uuid) {
        let now = format_dt(&now_pdt()).unwrap();
        sqlx::query(
            "INSERT INTO assembly_member_snapshot \
             (assembly_id, member_id, captured_at) VALUES (?, ?, ?)",
        )
        .bind(assembly_id.as_bytes().to_vec())
        .bind(member_id.as_bytes().to_vec())
        .bind(now)
        .execute(pool)
        .await
        .expect("insert snapshot");
    }

    async fn count_attendance_rows(pool: &SqlitePool, assembly_id: Uuid, member_id: Uuid) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM attendance WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(assembly_id.as_bytes().to_vec())
        .bind(member_id.as_bytes().to_vec())
        .fetch_one(pool)
        .await
        .expect("count")
    }

    /// Test 1: 5x UPSERT auf demselben Pair -> exakt 1 Row, deleted IS NULL,
    /// marked_by_user_id == letzter Caller (D-05 idempotency, ATTN-03).
    #[tokio::test]
    async fn test_upsert_present_idempotent_5x_creates_one_row() {
        let pool = setup_db().await;
        let aid = Uuid::new_v4();
        let mid = Uuid::new_v4();
        insert_assembly(&pool, aid).await;
        insert_member(&pool, mid, 100, "Max", "Mueller").await;
        insert_snapshot(&pool, aid, mid).await;

        let dao = AttendanceDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let tx = tx_dao.transaction().await.unwrap();
        for _ in 0..5 {
            dao.upsert_present(aid, mid, now_pdt(), "user1", tx.clone())
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();

        let count = count_attendance_rows(&pool, aid, mid).await;
        assert_eq!(count, 1, "5x UPSERT must yield exactly 1 row");

        let (deleted, marked_by) = sqlx::query_as::<_, (Option<String>, String)>(
            "SELECT deleted, marked_by_user_id FROM attendance \
             WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(aid.as_bytes().to_vec())
        .bind(mid.as_bytes().to_vec())
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert!(deleted.is_none(), "deleted must be NULL after toggle-on");
        assert_eq!(marked_by, "user1");
    }

    /// Test 2: Toggle-Off via soft_delete, dann Toggle-On via UPSERT
    /// resets deleted=NULL und überschreibt marked_by_user_id (D-06, D-09).
    #[tokio::test]
    async fn test_soft_delete_then_upsert_resets_deleted() {
        let pool = setup_db().await;
        let aid = Uuid::new_v4();
        let mid = Uuid::new_v4();
        insert_assembly(&pool, aid).await;
        insert_member(&pool, mid, 100, "Max", "Mueller").await;
        insert_snapshot(&pool, aid, mid).await;

        let dao = AttendanceDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        // 1) Toggle-On
        let tx = tx_dao.transaction().await.unwrap();
        dao.upsert_present(aid, mid, now_pdt(), "user1", tx.clone())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // 2) Toggle-Off
        let tx = tx_dao.transaction().await.unwrap();
        dao.soft_delete(aid, mid, now_pdt(), tx.clone())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let deleted_after_off: Option<String> = sqlx::query_scalar(
            "SELECT deleted FROM attendance WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(aid.as_bytes().to_vec())
        .bind(mid.as_bytes().to_vec())
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert!(
            deleted_after_off.is_some(),
            "deleted must be set after soft_delete"
        );

        // 3) Toggle-On again -> resets deleted, overrides marked_by_user_id
        let tx = tx_dao.transaction().await.unwrap();
        dao.upsert_present(aid, mid, now_pdt(), "user2", tx.clone())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let (deleted, marked_by) = sqlx::query_as::<_, (Option<String>, String)>(
            "SELECT deleted, marked_by_user_id FROM attendance \
             WHERE assembly_id = ? AND member_id = ?",
        )
        .bind(aid.as_bytes().to_vec())
        .bind(mid.as_bytes().to_vec())
        .fetch_one(&*pool)
        .await
        .unwrap();
        assert!(
            deleted.is_none(),
            "toggle-on after toggle-off must reset deleted to NULL"
        );
        assert_eq!(marked_by, "user2");
    }

    /// Test 3: soft_delete auf nicht-existierender Row -> Ok(()) (kein
    /// NotFound) (D-06 idempotency, ATTN-04).
    #[tokio::test]
    async fn test_soft_delete_on_nonexistent_row_is_ok() {
        let pool = setup_db().await;
        let dao = AttendanceDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        let tx = tx_dao.transaction().await.unwrap();
        // No INSERT prior: row does not exist.
        let result = dao
            .soft_delete(Uuid::new_v4(), Uuid::new_v4(), now_pdt(), tx.clone())
            .await;
        assert!(result.is_ok(), "soft_delete on missing row must return Ok");
        tx.commit().await.unwrap();
    }

    /// Test 4: list_members_for_assembly filters by snapshot membership
    /// AND by substring (case-insensitive) (D-24, D-25, ATTN-01).
    #[tokio::test]
    async fn test_list_members_for_assembly_filters_by_snapshot_and_substring() {
        let pool = setup_db().await;
        let aid = Uuid::new_v4();
        let mid_in = Uuid::new_v4(); // Mueller -- in snapshot
        let mid_out = Uuid::new_v4(); // Schmidt -- NOT in snapshot
        insert_assembly(&pool, aid).await;
        insert_member(&pool, mid_in, 100, "Max", "Mueller").await;
        insert_member(&pool, mid_out, 200, "Anna", "Schmidt").await;
        insert_snapshot(&pool, aid, mid_in).await;
        // Note: mid_out is NOT in the snapshot.

        let dao = AttendanceDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        // search=None -> only snapshot members.
        let tx = tx_dao.transaction().await.unwrap();
        let all = dao
            .list_members_for_assembly(aid, None, tx.clone())
            .await
            .unwrap();
        assert_eq!(all.len(), 1, "only snapshot members are returned");
        assert_eq!(all[0].last_name.as_ref(), "Mueller");
        assert!(!all[0].is_present, "no attendance row -> is_present=false");

        // search=Some("schmi") -> empty (Schmidt not in snapshot).
        let none = dao
            .list_members_for_assembly(aid, Some("schmi".to_string()), tx.clone())
            .await
            .unwrap();
        assert_eq!(none.len(), 0, "Schmidt not in snapshot -> no match");

        // search=Some("muell") -> 1 (case-insensitive ASCII).
        let one = dao
            .list_members_for_assembly(aid, Some("muell".to_string()), tx.clone())
            .await
            .unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].last_name.as_ref(), "Mueller");

        tx.commit().await.unwrap();
    }

    /// Test 5: count_present_by_assembly counts only deleted IS NULL rows
    /// (ASSY-04 stats accuracy after toggle-off).
    #[tokio::test]
    async fn test_count_present_by_assembly_excludes_soft_deleted() {
        let pool = setup_db().await;
        let aid = Uuid::new_v4();
        let mid_a = Uuid::new_v4();
        let mid_b = Uuid::new_v4();
        insert_assembly(&pool, aid).await;
        insert_member(&pool, mid_a, 100, "Max", "Mueller").await;
        insert_member(&pool, mid_b, 200, "Anna", "Schmidt").await;
        insert_snapshot(&pool, aid, mid_a).await;
        insert_snapshot(&pool, aid, mid_b).await;

        let dao = AttendanceDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool.clone());

        // Mark both present.
        let tx = tx_dao.transaction().await.unwrap();
        dao.upsert_present(aid, mid_a, now_pdt(), "user1", tx.clone())
            .await
            .unwrap();
        dao.upsert_present(aid, mid_b, now_pdt(), "user1", tx.clone())
            .await
            .unwrap();
        let count_two = dao
            .count_present_by_assembly(aid, tx.clone())
            .await
            .unwrap();
        assert_eq!(count_two, 2);

        // Soft-delete one.
        dao.soft_delete(aid, mid_a, now_pdt(), tx.clone())
            .await
            .unwrap();
        let count_one = dao
            .count_present_by_assembly(aid, tx.clone())
            .await
            .unwrap();
        assert_eq!(count_one, 1, "soft-deleted row must not be counted");

        tx.commit().await.unwrap();
    }

    /// Test 6: is_in_snapshot returns true iff (aid, mid) row exists in
    /// assembly_member_snapshot (D-27).
    #[tokio::test]
    async fn test_is_in_snapshot_true_false() {
        let pool = setup_db().await;
        let aid = Uuid::new_v4();
        let mid_in = Uuid::new_v4();
        let mid_out = Uuid::new_v4();
        insert_assembly(&pool, aid).await;
        insert_member(&pool, mid_in, 100, "Max", "Mueller").await;
        insert_member(&pool, mid_out, 200, "Anna", "Schmidt").await;
        insert_snapshot(&pool, aid, mid_in).await;

        let dao = AttendanceDaoImpl::new(pool.clone());
        let tx_dao = TransactionDaoImpl::new(pool);

        let tx = tx_dao.transaction().await.unwrap();
        let yes = dao.is_in_snapshot(aid, mid_in, tx.clone()).await.unwrap();
        assert!(yes, "mid_in is in snapshot");

        let no = dao.is_in_snapshot(aid, mid_out, tx.clone()).await.unwrap();
        assert!(!no, "mid_out is NOT in snapshot");
        tx.commit().await.unwrap();
    }
}
