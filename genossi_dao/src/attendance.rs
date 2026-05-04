use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;

/// AttendanceEntity -- D-01: 5 columns mirroring the attendance table.
///
/// **Lightweight join** between Assembly and Member with no own identity
/// (D-01: no id/version). Soft-delete-flip via `deleted` (D-09 -- first
/// productive use of the soft-delete slot in a GV-aggregate).
///
/// Lifecycle: Toggle-On overwrites with `deleted=NULL`, Toggle-Off sets
/// `deleted=Some(now())`. UPSERT-Reuse-Pattern (D-05) ensures exactly one
/// row per `(assembly_id, member_id)` pair.
///
/// **PII-Leak-Guard:** This entity intentionally has NO link to MemberEntity
/// fields (no email, no IBAN, no address). The reduced Helper-View uses the
/// separate `AttendanceMemberRow` projection (whitelist of 7 fields) -- see
/// ATTN-01 and Pitfall 6 of RESEARCH.md. Do NOT add a `From<&MemberEntity>`
/// conversion that would re-expose PII.
///
/// **Not Auditable** (D-08, ATTN-05) -- no `Auditable` impl, no `audit_fields()`.
/// The reasoning: the Genossenschaftsverband only requires the count of
/// attendees in the GV protocol, not the act of marking each individual.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceEntity {
    pub assembly_id: Uuid,
    pub member_id: Uuid,
    pub marked_at: time::PrimitiveDateTime,
    pub marked_by_user_id: Arc<str>,
    pub deleted: Option<time::PrimitiveDateTime>,
}

/// Reduced 7-field projection for the Helper-View (D-24, ATTN-01).
/// Returned by `list_members_for_assembly`. NOT a full member record --
/// no PII fields (email, address, bank, etc.).
///
/// **PII-Leak-Guard:** SELECT-Whitelist of exactly these 7 columns. Any
/// future MemberEntity expansion (e.g. new PII field) will NOT leak into
/// this projection because the SQL is explicit -- never `SELECT m.*`.
/// See T-03-01-02 in the threat model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceMemberRow {
    pub member_id: Uuid,
    pub member_number: i64,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Arc<str>>,
    pub title: Option<Arc<str>>,
    pub is_present: bool,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AttendanceDao {
    type Transaction: crate::Transaction;

    /// D-05 atomic toggle-on via SQLite UPSERT.
    ///
    /// Idempotent: 5x call -> 5x Ok(()) -> exactly one row in attendance.
    /// On conflict (existing row) overrides `marked_at`, `marked_by_user_id`,
    /// and resets `deleted` to NULL (D-09 -- toggle-on overrides a previous
    /// soft-delete).
    async fn upsert_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        marked_at: time::PrimitiveDateTime,
        marked_by_user_id: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// D-06 atomic toggle-off via UPDATE soft-delete.
    ///
    /// Idempotent: 5x call on non-existent row -> 5x Ok(()) (No-Op).
    /// `rows_affected` is intentionally ignored.
    async fn soft_delete(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        deleted_at: time::PrimitiveDateTime,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    /// ATTN-01 + ATTN-02 + D-25: reduced member view filtered by snapshot
    /// membership, with optional substring filter.
    ///
    /// Single SQL with JOIN snapshot + LEFT JOIN attendance for `is_present`.
    /// Substring search is executed in the DAO (LIKE COLLATE NOCASE on
    /// `last_name||' '||first_name` and `member_number`-as-text).
    ///
    /// Note: `search` is `Option<String>` rather than `Option<&str>` because
    /// `async_trait` + `automock` do not support borrowed-data parameters
    /// without explicit lifetime annotations on every implementor. The DAO
    /// internally wraps the string with `%...%` LIKE-pattern, so it always
    /// allocates a new String anyway -- accepting an owned String avoids a
    /// double allocation and avoids the lifetime gymnastics. Service-layer
    /// callers pass `search.map(String::from)` from their `Option<&str>`.
    async fn list_members_for_assembly(
        &self,
        assembly_id: Uuid,
        search: Option<String>,
        tx: Self::Transaction,
    ) -> Result<Arc<[AttendanceMemberRow]>, DaoError>;

    /// ASSY-04: `present` counter for stats endpoint.
    /// Counts rows in attendance with `deleted IS NULL` for the given assembly.
    async fn count_present_by_assembly(
        &self,
        assembly_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<u64, DaoError>;

    /// D-27: snapshot membership check.
    ///
    /// Returns true if `member_id` is in the snapshot of `assembly_id`.
    /// Co-located here (instead of in `AssemblyMemberSnapshotDao`) so
    /// `mark_present`/`mark_absent` can call a single DAO without a
    /// round-trip through two DAOs.
    async fn is_in_snapshot(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<bool, DaoError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdt() -> time::PrimitiveDateTime {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 4).unwrap();
        time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
    }

    #[test]
    fn test_attendance_entity_has_exactly_five_fields() {
        // D-01: AttendanceEntity has 5 columns -- no id, no version.
        // Construction here is the compile-time contract.
        let entity = AttendanceEntity {
            assembly_id: Uuid::new_v4(),
            member_id: Uuid::new_v4(),
            marked_at: pdt(),
            marked_by_user_id: Arc::from("helper:abc"),
            deleted: None,
        };
        assert_eq!(entity.marked_by_user_id.as_ref(), "helper:abc");
        assert!(entity.deleted.is_none());
    }

    #[test]
    fn test_attendance_member_row_has_exactly_seven_fields() {
        // D-24 / ATTN-01: AttendanceMemberRow has 7 fields -- whitelist
        // against MemberEntity to prevent PII leak.
        let row = AttendanceMemberRow {
            member_id: Uuid::new_v4(),
            member_number: 42,
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mueller"),
            salutation: Some(Arc::from("Herr")),
            title: None,
            is_present: true,
        };
        assert_eq!(row.member_number, 42);
        assert!(row.is_present);
        assert_eq!(row.last_name.as_ref(), "Mueller");
        assert!(row.title.is_none());
    }

    #[test]
    fn test_mock_attendance_dao_can_be_constructed() {
        // Verifies that #[automock(type Transaction = crate::MockTransaction;)]
        // is correctly wired -- if the macro is mis-applied this test won't
        // compile.
        let _mock = MockAttendanceDao::new();
    }
}
