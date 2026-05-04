//! Service-layer trait + domain types for the Attendance aggregate.
//!
//! Phase 3 Plan 04 wires the `AttendanceService` trait, the `AttendanceStats`
//! domain type, and a #[automock]-derived MockAttendanceService used by the
//! REST layer's tests in Plan 06. Plan 05 implements the trait
//! (`AttendanceServiceImpl`); the REST handlers in Plan 06 type-bind directly
//! against this trait.

use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use genossi_dao::attendance::AttendanceMemberRow;

use crate::permission::Authentication;
use crate::ServiceError;

/// Aggregate counter for the live attendance counter (ASSY-04).
///
/// `present`: Number of members with an active attendance row
///   (attendance.deleted IS NULL) for the assembly.
/// `total`: Number of members in the Member-Universe-Snapshot of the assembly
///   (`assembly_member_snapshot.count_by_assembly_id`) -- defines the stable
///   `Y` in the "X von Y aktiven Mitgliedern" counter (Phase 1 D-XX).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceStats {
    pub present: u64,
    pub total: u64,
}

/// AttendanceService -- helper/board API for GV attendance recording.
///
/// **Permission funnel** (D-17/D-18): every method MUST call
/// `check_assembly_access(assembly_id, ctx, tx)` as its first step in the
/// implementation (prevents the "endpoint forgot the status check" bug).
/// Implementation lives in `AttendanceServiceImpl` (Plan 05).
///
/// **No audit** (D-08, ATTN-05): attendance operations are intentionally NOT
/// recorded in the audit hashchain. The board's post-close edit (ASSY-06)
/// uses the same service path -- also without audit.
///
/// **PII guard** (D-24, ATTN-01): `list_members` returns
/// `Arc<[AttendanceMemberRow]>` from the DAO -- a 7-field whitelist projection
/// (member_number, first_name, last_name, salutation, title, is_present,
/// member_id). The REST layer converts to `AttendanceMemberTO` (also 7 fields)
/// in Plan 06; no PII paths through MemberTO are allowed.
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AttendanceService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// ATTN-01 + ATTN-02: reduced member view with optional substring filter.
    ///
    /// Returns: list of `AttendanceMemberRow` (DAO projection with the 7
    /// allowed fields).
    /// Errors: `PermissionDenied` (helper with wrong assembly_id, GV not Open,
    /// or non-admin user); `EntityNotFound` (assembly_id does not exist).
    async fn list_members(
        &self,
        assembly_id: Uuid,
        search: Option<String>,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[AttendanceMemberRow]>, ServiceError>;

    /// ATTN-03: idempotent toggle-on (UPSERT -- D-05).
    ///
    /// Errors: `PermissionDenied`, `EntityNotFound` (assembly_id or member_id
    /// not in snapshot -- D-27).
    async fn mark_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    /// ATTN-04: idempotent toggle-off (UPDATE soft-delete -- D-06).
    ///
    /// Idempotent even when no row exists. Snapshot check (D-27) still applies.
    /// Errors: `PermissionDenied`, `EntityNotFound` (assembly_id or member_id
    /// not in snapshot).
    async fn mark_absent(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    /// ASSY-04: live counter `{present, total}`.
    ///
    /// Both helpers and the board may see the counter (CONTEXT Discretion 7).
    /// Errors: `PermissionDenied`, `EntityNotFound`.
    async fn stats(
        &self,
        assembly_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<AttendanceStats, ServiceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attendance_stats_constructible() {
        let stats = AttendanceStats {
            present: 5,
            total: 10,
        };
        assert_eq!(stats.present, 5);
        assert_eq!(stats.total, 10);
    }

    #[test]
    fn test_mock_attendance_service_compiles() {
        // Compile-only: ensure #[automock] generates MockAttendanceService.
        let _mock = MockAttendanceService::new();
    }

    #[test]
    fn test_mock_attendance_service_can_setup_expectations() {
        // Compile-time + setup-only verification: the #[automock]-generated
        // mock builder must expose `expect_stats()` (and analogous builders
        // for the other 3 methods). We don't invoke the mocked method here
        // because genossi_service has no tokio dev-dep; Plan 06's REST tests
        // exercise the actual await path.
        let mut mock = MockAttendanceService::new();
        mock.expect_stats().returning(|_aid, _ctx| {
            Ok(AttendanceStats {
                present: 0,
                total: 0,
            })
        });
        mock.expect_list_members()
            .returning(|_aid, _search, _ctx| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));
        mock.expect_mark_present().returning(|_aid, _mid, _ctx| Ok(()));
        mock.expect_mark_absent().returning(|_aid, _mid, _ctx| Ok(()));
        // The mock instance is dropped without invocation -- mockall does not
        // require expectations to be exercised when no `.times(...)` is set.
    }
}
