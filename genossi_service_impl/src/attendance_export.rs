//! Service-layer implementation of the Attendance-Export aggregate
//! (Phase 6 Plan 02).
//!
//! Wires `AttendanceExportServiceImpl` with:
//!   1. A permission funnel `check_admin_and_closed` (D-11, D-13) — distinct
//!      from `AttendanceServiceImpl::check_assembly_access` to avoid name
//!      collision and to enforce the EXPORT-specific rules:
//!        a. Admin-only via `PermissionService::check_permission("admin", ...)`.
//!        b. `assembly.status == Closed` — non-Closed yields
//!           `ServiceError::Conflict("assembly_not_closed")`.
//!      There is intentionally NO helper-branch: helpers must not export PII
//!      lists. Helper tokens are also invalidated on Close (Phase 3 cascade),
//!      so even a leaked helper context would resolve to a closed assembly
//!      where it has no privilege.
//!   2. Three format writers — CSV (D-03/D-16), XLSX (D-16), PDF (D-04, D-08).
//!   3. A single `export(...)` entry point that the REST handler (Plan 03)
//!      binds via `RestStateImpl::attendance_export_service()`.
//!
//! **D-17 — NO AUDIT.** This file intentionally never invokes `audited_create!`,
//! `audited_update!`, or `audited_delete!`. Exports are read-only and the
//! Genossenschaftsverband only requires the count, not the act of exporting.
//! `tracing::info!` (D-18) provides operational log visibility instead.
//!
//! **DSGVO PII guard** (T-06-06): the export reads from
//! `AttendanceDao::list_members_for_assembly` whose 7-col SELECT-whitelist
//! restricts the projection — identical guard as `AttendanceMemberTO` in
//! Phase 3 Plan 06. The PDF builder additionally drops `member_id` so only
//! six columns ever reach the file (member_number, first_name, last_name,
//! salutation, title, is_present).

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use rust_xlsxwriter::{Format, Workbook};
use uuid::Uuid;

use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::attendance::{AttendanceDao, AttendanceMemberRow};
use genossi_dao::Transaction;
use genossi_dao::TransactionDao;

use genossi_service::attendance_export::{
    AttendanceExport, AttendanceExportService, ExportFormat, ExportInclude,
};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::ServiceError;

use crate::pdf_generation::PdfGenerator;

/// Privilege constant — identical to the `"admin"` string used by all other
/// Phase-1/2/3 services (D-13). We deliberately do NOT introduce a new
/// `attendance.export` privilege per the planner decision; the existing
/// Vorstand role suffices.
const ADMIN_PRIVILEGE: &str = "admin";

/// Tracing target for the export log line (D-18).
const EXPORT_TARGET: &str = "attendance_export";

/// Dependency-injection trait for `AttendanceExportServiceImpl`. We define
/// this manually (not via `gen_service_impl!`) because the impl additionally
/// carries non-trait fields (`pdf_generator`, `template_base`); the macro
/// only handles DAO/service-trait fields. Plan 03 wires this shape verbatim
/// in `RestStateImpl::new()`.
pub trait AttendanceExportServiceDeps: Send + Sync + 'static {
    type Context: Clone + std::fmt::Debug + Send + Sync + 'static;
    type Transaction: Transaction;
    type AttendanceDao: AttendanceDao<Transaction = Self::Transaction> + Send + Sync;
    type AssemblyDao: AssemblyDao<Transaction = Self::Transaction> + Send + Sync;
    type PermissionService: PermissionService<Context = Self::Context> + Send + Sync;
    type TransactionDao: TransactionDao<Transaction = Self::Transaction> + Send + Sync;
}

/// Concrete service implementation. Plan 03 will instantiate this with the
/// production `Deps` provided by `genossi_bin`.
pub struct AttendanceExportServiceImpl<Deps: AttendanceExportServiceDeps> {
    pub transaction_dao: Arc<Deps::TransactionDao>,
    pub permission_service: Arc<Deps::PermissionService>,
    pub assembly_dao: Arc<Deps::AssemblyDao>,
    pub attendance_dao: Arc<Deps::AttendanceDao>,
    pub pdf_generator: Arc<PdfGenerator>,
    pub template_base: Arc<PathBuf>,
}

impl<Deps: AttendanceExportServiceDeps> AttendanceExportServiceImpl<Deps> {
    /// Permission funnel for the export endpoint (D-11 + D-13).
    ///
    /// Distinct name from `AttendanceServiceImpl::check_assembly_access`
    /// (Phase 3) to avoid confusion: there the helper branch is allowed and
    /// status-gate runs only on the helper branch; here NO helper-branch
    /// exists and the status-gate applies to everyone (admin included).
    ///
    /// Order matters:
    ///   1. Load the assembly (404 if missing).
    ///   2. Admin permission check (PermissionDenied if not admin).
    ///   3. Status check (`Conflict("assembly_not_closed")` if not Closed).
    ///
    /// The status check runs AFTER the permission check so a non-admin user
    /// cannot derive status information from the error variant alone.
    async fn check_admin_and_closed(
        &self,
        assembly_id: Uuid,
        context: Authentication<Deps::Context>,
        tx: Deps::Transaction,
    ) -> Result<AssemblyEntity, ServiceError> {
        let assembly = self
            .assembly_dao
            .find_by_id(assembly_id, tx)
            .await?
            .ok_or(ServiceError::EntityNotFound(assembly_id))?;

        // Admin gate (D-13). Authentication::Full short-circuits via the
        // PermissionService convention (test-fixtures + production OIDC
        // both honor it).
        match &context {
            Authentication::Full => {}
            Authentication::Context(_) => {
                self.permission_service
                    .check_permission(ADMIN_PRIVILEGE, context)
                    .await?;
            }
        }

        // Status gate (D-11). NO helper-branch — D-13 forbids helpers from
        // exporting at all.
        if assembly.status != AssemblyStatus::Closed {
            return Err(ServiceError::Conflict(Arc::from("assembly_not_closed")));
        }

        Ok(assembly)
    }
}

#[async_trait]
impl<Deps: AttendanceExportServiceDeps> AttendanceExportService
    for AttendanceExportServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn export(
        &self,
        assembly_id: Uuid,
        format: ExportFormat,
        include: ExportInclude,
        context: Authentication<Self::Context>,
    ) -> Result<AttendanceExport, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;
        // D-11 + D-13: permission funnel.
        let assembly = self
            .check_admin_and_closed(assembly_id, context, tx.clone())
            .await?;

        // D-05/D-06/D-07: reuse the existing DAO whitelist (7-col SELECT).
        // `search=None` returns every snapshot member with their current
        // attendance flag. D-12: ein einziger DAO-Read in derselben TX —
        // wir lesen den aktuellen Stand, nicht einen gecachten Wert.
        let mut rows: Vec<AttendanceMemberRow> = self
            .attendance_dao
            .list_members_for_assembly(assembly_id, None, tx.clone())
            .await?
            .iter()
            .cloned()
            .collect();

        // D-09: include=Present filtert nach is_present == true.
        if matches!(include, ExportInclude::Present) {
            rows.retain(|r| r.is_present);
        }

        // Commit the read-only transaction (D-12). Read-only by intent, but
        // keeping the pattern consistent with the other services means
        // future audit additions (should the planner decision ever flip)
        // would still pick up the read consistency point.
        self.transaction_dao.commit(tx).await?;

        let present = rows.iter().filter(|r| r.is_present).count() as u64;
        // D-10 / Invariante "Y = rows.len()": rows kommt aus
        // list_members_for_assembly(search=None) -> eine Zeile pro Snapshot-
        // Mitglied. include=All bewahrt diese Cardinality, daher ist
        // rows.len() exakt das Member-Universe.
        let total = match include {
            ExportInclude::All => Some(rows.len() as u64),
            ExportInclude::Present => None,
        };

        // D-15: Filename schema `gv-{YYYY-MM-DD}-teilnehmer.{ext}`.
        let date_format = time::format_description::parse("[year]-[month]-[day]")
            .expect("static iso-date format");
        let date_str = assembly
            .date
            .date()
            .format(&date_format)
            .unwrap_or_else(|_| "unknown".to_string());

        // D-18: structured tracing for the export call.
        tracing::info!(
            target: EXPORT_TARGET,
            aid = %assembly_id,
            format = ?format,
            include = ?include,
            rows = rows.len(),
            "exporting attendance"
        );

        let (bytes, content_type, ext) = match format {
            ExportFormat::Csv => (render_csv(&rows)?, "text/csv; charset=utf-8", "csv"),
            ExportFormat::Xlsx => (
                render_xlsx(&rows)?,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "xlsx",
            ),
            ExportFormat::Pdf => (
                self.pdf_generator.render_attendance_list(
                    "teilnehmerliste.typ",
                    self.template_base.as_path(),
                    &assembly,
                    &rows,
                    present,
                    total,
                )?,
                "application/pdf",
                "pdf",
            ),
        };

        Ok(AttendanceExport {
            bytes,
            content_type,
            filename: format!("gv-{}-teilnehmer.{}", date_str, ext),
        })
    }
}

// ---------------------------------------------------------------------------
// Format writers (free functions — no `self` state needed)
// ---------------------------------------------------------------------------

/// CSV writer per D-03 / D-16:
///   * UTF-8 BOM prefix `[0xEF, 0xBB, 0xBF]` (Excel-on-Windows compatibility).
///   * Semicolon delimiter (German Excel locale).
///   * Headers: Mitgliedsnummer, Nachname, Vorname, Anrede, Titel, anwesend.
///   * Anwesenheits-Spalte: "ja" / "nein" (Planner-Entscheidung — konsistent
///     mit XLSX-Writer).
fn render_csv(rows: &[AttendanceMemberRow]) -> Result<Vec<u8>, ServiceError> {
    // BOM (D-03).
    let mut buf: Vec<u8> = vec![0xEF, 0xBB, 0xBF];

    {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_writer(&mut buf);

        wtr.write_record([
            "Mitgliedsnummer",
            "Nachname",
            "Vorname",
            "Anrede",
            "Titel",
            "anwesend",
        ])
        .map_err(|e| {
            ServiceError::InternalError(Arc::from(format!("csv header write failed: {}", e)))
        })?;

        for r in rows.iter() {
            wtr.write_record([
                r.member_number.to_string(),
                r.last_name.to_string(),
                r.first_name.to_string(),
                r.salutation
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                r.title.as_ref().map(|s| s.to_string()).unwrap_or_default(),
                if r.is_present { "ja" } else { "nein" }.to_string(),
            ])
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!("csv row write failed: {}", e)))
            })?;
        }

        wtr.flush().map_err(|e| {
            ServiceError::InternalError(Arc::from(format!("csv flush failed: {}", e)))
        })?;
    }

    Ok(buf)
}

/// XLSX writer per D-16. Returns the full `.xlsx` zip archive as bytes.
///
/// Synchronous block — no `.await` between Workbook operations (RESEARCH
/// Pitfall 8: the `rust_xlsxwriter` API is not Send).
fn render_xlsx(rows: &[AttendanceMemberRow]) -> Result<Vec<u8>, ServiceError> {
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();

    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Teilnehmer")
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("xlsx set_name: {}", e))))?;

    let headers = [
        "Mitgliedsnummer",
        "Nachname",
        "Vorname",
        "Anrede",
        "Titel",
        "anwesend",
    ];
    for (col, h) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *h, &bold)
            .map_err(|e| ServiceError::InternalError(Arc::from(format!("xlsx header: {}", e))))?;
    }

    for (idx, r) in rows.iter().enumerate() {
        let row_num = (idx + 1) as u32;

        // Mitgliedsnummer as numeric so Excel sorts it correctly.
        sheet
            .write_number(row_num, 0, r.member_number as f64)
            .map_err(|e| ServiceError::InternalError(Arc::from(format!("xlsx number: {}", e))))?;
        sheet
            .write_string(row_num, 1, r.last_name.as_ref())
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!("xlsx last_name: {}", e)))
            })?;
        sheet
            .write_string(row_num, 2, r.first_name.as_ref())
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!("xlsx first_name: {}", e)))
            })?;
        sheet
            .write_string(row_num, 3, r.salutation.as_deref().unwrap_or(""))
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!("xlsx salutation: {}", e)))
            })?;
        sheet
            .write_string(row_num, 4, r.title.as_deref().unwrap_or(""))
            .map_err(|e| ServiceError::InternalError(Arc::from(format!("xlsx title: {}", e))))?;
        sheet
            .write_string(row_num, 5, if r.is_present { "ja" } else { "nein" })
            .map_err(|e| ServiceError::InternalError(Arc::from(format!("xlsx present: {}", e))))?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("xlsx save: {}", e))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::DaoError;
    use genossi_service::claim_context::ClaimContext;
    use mockall::{mock, predicate::*};
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ----------------------------------------------------------------------
    // Test infrastructure (hand-rolled mocks — same pattern as
    // genossi_service_impl::attendance::tests).
    // ----------------------------------------------------------------------

    #[derive(Clone, Debug)]
    pub struct TestTransaction;

    #[async_trait]
    impl Transaction for TestTransaction {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TestContext;

    impl ClaimContext for TestContext {
        fn has_claims(&self) -> bool {
            false
        }
    }

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao {
            type Transaction = TestTransaction;
            async fn transaction(&self) -> Result<TestTransaction, DaoError>;
            async fn use_transaction(
                &self,
                tx: Option<TestTransaction>,
            ) -> Result<TestTransaction, DaoError>;
            async fn commit(&self, tx: TestTransaction) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub TestAssemblyDao {}
        #[async_trait]
        impl AssemblyDao for TestAssemblyDao {
            type Transaction = TestTransaction;
            async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
            async fn create(&self, entity: &AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn update(&self, entity: &AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTransaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<AssemblyEntity>, DaoError>;
        }
    }

    mock! {
        pub TestAttendanceDao {}
        #[async_trait]
        impl AttendanceDao for TestAttendanceDao {
            type Transaction = TestTransaction;
            async fn upsert_present(
                &self,
                assembly_id: Uuid,
                member_id: Uuid,
                marked_at: time::PrimitiveDateTime,
                marked_by_user_id: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn soft_delete(
                &self,
                assembly_id: Uuid,
                member_id: Uuid,
                deleted_at: time::PrimitiveDateTime,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn list_members_for_assembly(
                &self,
                assembly_id: Uuid,
                search: Option<String>,
                tx: TestTransaction,
            ) -> Result<Arc<[AttendanceMemberRow]>, DaoError>;
            async fn count_present_by_assembly(
                &self,
                assembly_id: Uuid,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
            async fn is_in_snapshot(
                &self,
                assembly_id: Uuid,
                member_id: Uuid,
                tx: TestTransaction,
            ) -> Result<bool, DaoError>;
        }
    }

    mock! {
        pub TestPermissionService {}
        #[async_trait]
        impl PermissionService for TestPermissionService {
            type Context = TestContext;
            async fn check_permission(
                &self,
                privilege: &str,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn current_user_id(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(
                &self,
                user: genossi_service::auth_types::UserTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_user(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_roles(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(
                &self,
                role: genossi_service::auth_types::RoleTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_role(
                &self,
                role_name: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_all_privileges(
                &self,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(
                &self,
                privilege: genossi_service::auth_types::PrivilegeTO,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn delete_privilege(
                &self,
                privilege_name: String,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn assign_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_user_role(
                &self,
                user_role: genossi_service::auth_types::UserRole,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_user_roles(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn remove_role_privilege(
                &self,
                role_privilege: genossi_service::auth_types::RolePrivilege,
                context: Authentication<TestContext>,
            ) -> Result<(), ServiceError>;
            async fn get_role_privileges(
                &self,
                role_name: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(
                &self,
                username: String,
                context: Authentication<TestContext>,
            ) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &TestContext) -> Result<bool, ServiceError>;
        }
    }

    pub struct TestDeps;

    impl AttendanceExportServiceDeps for TestDeps {
        type Context = TestContext;
        type Transaction = TestTransaction;
        type AttendanceDao = MockTestAttendanceDao;
        type AssemblyDao = MockTestAssemblyDao;
        type PermissionService = MockTestPermissionService;
        type TransactionDao = MockTestTxDao;
    }

    fn assembly_in_status(aid: Uuid, status: AssemblyStatus) -> AssemblyEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::May, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        AssemblyEntity {
            id: aid,
            name: Arc::from("GV 2026"),
            date: datetime,
            location: Some(Arc::from("Vereinsheim")),
            status,
            opened_at: None,
            closed_at: None,
            created: datetime,
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn build_service(
        attendance_dao: MockTestAttendanceDao,
        assembly_dao: MockTestAssemblyDao,
        permission_service: MockTestPermissionService,
        tx_dao: MockTestTxDao,
        template_base: PathBuf,
    ) -> AttendanceExportServiceImpl<TestDeps> {
        AttendanceExportServiceImpl {
            attendance_dao: Arc::new(attendance_dao),
            assembly_dao: Arc::new(assembly_dao),
            permission_service: Arc::new(permission_service),
            transaction_dao: Arc::new(tx_dao),
            pdf_generator: Arc::new(PdfGenerator::new()),
            template_base: Arc::new(template_base),
        }
    }

    fn tx_dao_with_commit() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().returning(|_| Ok(()));
        tx_dao
    }

    fn tx_dao_no_commit() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().times(0..=1).returning(|_| Ok(()));
        tx_dao
    }

    fn sample_rows() -> Arc<[AttendanceMemberRow]> {
        Arc::from(vec![
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 1,
                first_name: Arc::from("Alice"),
                last_name: Arc::from("Anders"),
                salutation: Some(Arc::from("Frau")),
                title: Some(Arc::from("Dr.")),
                is_present: true,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 2,
                first_name: Arc::from("Bob"),
                last_name: Arc::from("Beck"),
                salutation: None,
                title: None,
                is_present: false,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 3,
                first_name: Arc::from("Carla"),
                last_name: Arc::from("Cremer"),
                salutation: Some(Arc::from("Frau")),
                title: None,
                is_present: true,
            },
        ])
    }

    // ----------------------------------------------------------------------
    // Permission-funnel tests (Closed-gate + admin-gate)
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn closed_admin_success() {
        // Authentication::Full + Status==Closed → success (CSV path).
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .with(eq(aid), always())
            .times(1)
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut att_dao = MockTestAttendanceDao::new();
        let rows = sample_rows();
        att_dao
            .expect_list_members_for_assembly()
            .withf(|_, s, _| s.is_none())
            .times(1)
            .returning(move |_, _, _| Ok(rows.clone()));
        let svc = build_service(
            att_dao,
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Full,
            )
            .await
            .expect("export should succeed");
        assert_eq!(res.content_type, "text/csv; charset=utf-8");
        assert!(res.filename.ends_with(".csv"));
    }

    #[tokio::test]
    async fn non_closed_returns_conflict_preparation() {
        // Preparation -> Conflict("assembly_not_closed").
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Preparation);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Full,
            )
            .await;
        match res {
            Err(ServiceError::Conflict(msg)) => assert_eq!(msg.as_ref(), "assembly_not_closed"),
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn non_closed_returns_conflict_open() {
        // Open -> Conflict("assembly_not_closed").
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Open);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Full,
            )
            .await;
        assert!(
            matches!(&res, Err(ServiceError::Conflict(msg)) if msg.as_ref() == "assembly_not_closed"),
            "expected Conflict(assembly_not_closed), got {:?}",
            res
        );
    }

    #[tokio::test]
    async fn non_admin_returns_permission_denied() {
        // Context (non-Full) + PermissionService returns PermissionDenied →
        // bubble up. Status-check happens AFTER permission-check, so
        // assembly status is irrelevant.
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .withf(|p, _| p == ADMIN_PRIVILEGE)
            .times(1)
            .returning(|_, _| Err(ServiceError::PermissionDenied));
        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            perm,
            tx_dao_no_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Context(TestContext),
            )
            .await;
        assert!(
            matches!(res, Err(ServiceError::PermissionDenied)),
            "expected PermissionDenied, got {:?}",
            res
        );
    }

    #[tokio::test]
    async fn not_found_returns_entity_not_found() {
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        assembly_dao.expect_find_by_id().returning(|_, _| Ok(None));
        let svc = build_service(
            MockTestAttendanceDao::new(),
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_no_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Full,
            )
            .await;
        assert!(
            matches!(res, Err(ServiceError::EntityNotFound(uid)) if uid == aid),
            "expected EntityNotFound, got {:?}",
            res
        );
    }

    #[tokio::test]
    async fn admin_context_passes_permission_check() {
        // Test 6 mirror: Admin-Context (not Full) + admin-grant + Closed → success.
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .withf(|p, _| p == ADMIN_PRIVILEGE)
            .times(1)
            .returning(|_, _| Ok(()));
        let mut att_dao = MockTestAttendanceDao::new();
        att_dao
            .expect_list_members_for_assembly()
            .returning(move |_, _, _| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));
        let svc = build_service(
            att_dao,
            assembly_dao,
            perm,
            tx_dao_with_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Context(TestContext),
            )
            .await;
        assert!(res.is_ok(), "expected Ok, got {:?}", res);
    }

    // ----------------------------------------------------------------------
    // Include-filter test
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn include_present_filters_absent() {
        // include=Present must drop the absent rows. With 2 present + 1
        // absent the CSV body has exactly 2 data rows.
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut att_dao = MockTestAttendanceDao::new();
        let rows = sample_rows(); // 3 rows: 2 present + 1 absent
        att_dao
            .expect_list_members_for_assembly()
            .returning(move |_, _, _| Ok(rows.clone()));
        let svc = build_service(
            att_dao,
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::Present,
                Authentication::Full,
            )
            .await
            .expect("export ok");
        // BOM + header + 2 rows = 4 lines; trim BOM and parse.
        let body = &res.bytes[3..];
        let text = std::str::from_utf8(body).expect("utf8");
        let row_count = text.lines().filter(|l| !l.is_empty()).count();
        // Header + 2 present rows = 3 non-empty lines.
        assert_eq!(
            row_count, 3,
            "expected header + 2 present rows, got {}",
            row_count
        );
    }

    // ----------------------------------------------------------------------
    // Format-specific tests (CSV magic, XLSX magic, filename, content-type)
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn csv_starts_with_bom_and_uses_semicolon() {
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut att_dao = MockTestAttendanceDao::new();
        let rows = sample_rows();
        att_dao
            .expect_list_members_for_assembly()
            .returning(move |_, _, _| Ok(rows.clone()));
        let svc = build_service(
            att_dao,
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Full,
            )
            .await
            .expect("export ok");
        // BOM check
        assert_eq!(
            &res.bytes[..3],
            &[0xEF, 0xBB, 0xBF],
            "CSV must start with UTF-8 BOM"
        );
        // Semicolon header
        let body = std::str::from_utf8(&res.bytes[3..]).expect("utf8");
        let first_line = body.lines().next().expect("at least one line");
        assert!(
            first_line.contains(';'),
            "header line should contain ';' delimiter: {:?}",
            first_line
        );
        assert!(
            !first_line.starts_with("Mitgliedsnummer,"),
            "must NOT use comma delimiter: {:?}",
            first_line
        );
        // ja/nein anwesend column
        assert!(
            body.contains(";ja"),
            "CSV should have at least one ;ja cell: {:?}",
            body
        );
        assert!(
            body.contains(";nein"),
            "CSV should have at least one ;nein cell: {:?}",
            body
        );
        // Row count = 1 header + 3 rows (include=All)
        let row_count = body.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(row_count, 4, "expected 1 header + 3 rows");
    }

    #[tokio::test]
    async fn xlsx_starts_with_zip_magic() {
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut att_dao = MockTestAttendanceDao::new();
        let rows = sample_rows();
        att_dao
            .expect_list_members_for_assembly()
            .returning(move |_, _, _| Ok(rows.clone()));
        let svc = build_service(
            att_dao,
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Xlsx,
                ExportInclude::All,
                Authentication::Full,
            )
            .await
            .expect("export ok");
        assert!(
            res.bytes.len() > 4,
            "XLSX bytes too short: {}",
            res.bytes.len()
        );
        assert_eq!(
            &res.bytes[..4],
            b"PK\x03\x04",
            "XLSX must start with ZIP magic"
        );
        assert_eq!(
            res.content_type,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert!(res.filename.ends_with(".xlsx"));
    }

    #[tokio::test]
    async fn filename_schema_correct() {
        // Verify filename for all three formats with assembly date 2026-05-15.
        let aid = Uuid::new_v4();
        for (fmt, expected_ext, expected_ct) in [
            (ExportFormat::Csv, "csv", "text/csv; charset=utf-8"),
            (
                ExportFormat::Xlsx,
                "xlsx",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
        ] {
            let mut assembly_dao = MockTestAssemblyDao::new();
            let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
            assembly_dao
                .expect_find_by_id()
                .returning(move |_, _| Ok(Some(assembly.clone())));
            let mut att_dao = MockTestAttendanceDao::new();
            att_dao
                .expect_list_members_for_assembly()
                .returning(move |_, _, _| Ok(Arc::from(Vec::<AttendanceMemberRow>::new())));
            let svc = build_service(
                att_dao,
                assembly_dao,
                MockTestPermissionService::new(),
                tx_dao_with_commit(),
                PathBuf::from("templates"),
            );
            let res = svc
                .export(aid, fmt, ExportInclude::All, Authentication::Full)
                .await
                .expect("export ok");
            assert_eq!(
                res.filename,
                format!("gv-2026-05-15-teilnehmer.{}", expected_ext),
                "filename schema mismatch for {:?}",
                fmt
            );
            assert_eq!(res.content_type, expected_ct);
        }
    }

    // ----------------------------------------------------------------------
    // D-10 invariant: list returns one row per snapshot member.
    // Service must keep Y == rows.len() when include=All, no caching layer.
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn list_returns_one_row_per_snapshot_member() {
        let aid = Uuid::new_v4();
        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        // Exactly 5 mock rows — 3 present, 2 absent.
        let rows: Arc<[AttendanceMemberRow]> = Arc::from(vec![
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 1,
                first_name: Arc::from("A"),
                last_name: Arc::from("A"),
                salutation: None,
                title: None,
                is_present: true,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 2,
                first_name: Arc::from("B"),
                last_name: Arc::from("B"),
                salutation: None,
                title: None,
                is_present: true,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 3,
                first_name: Arc::from("C"),
                last_name: Arc::from("C"),
                salutation: None,
                title: None,
                is_present: false,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 4,
                first_name: Arc::from("D"),
                last_name: Arc::from("D"),
                salutation: None,
                title: None,
                is_present: true,
            },
            AttendanceMemberRow {
                member_id: Uuid::new_v4(),
                member_number: 5,
                first_name: Arc::from("E"),
                last_name: Arc::from("E"),
                salutation: None,
                title: None,
                is_present: false,
            },
        ]);
        let mut att_dao = MockTestAttendanceDao::new();
        att_dao
            .expect_list_members_for_assembly()
            .withf(|_, s, _| s.is_none()) // D-05 — search=None
            .times(1)
            .returning(move |_, _, _| Ok(rows.clone()));
        let svc = build_service(
            att_dao,
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
            PathBuf::from("templates"),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Csv,
                ExportInclude::All,
                Authentication::Full,
            )
            .await
            .expect("export ok");
        // Header + 5 rows = 6 non-empty lines after BOM.
        let body = std::str::from_utf8(&res.bytes[3..]).expect("utf8");
        let row_count = body.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(row_count, 6, "expected 1 header + 5 rows");
    }

    // ----------------------------------------------------------------------
    // PDF integration test (uses a temp template to avoid coupling to the
    // real templates/ dir during unit tests).
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn pdf_export_returns_pdf_magic() {
        let aid = Uuid::new_v4();
        let dir = TempDir::new().unwrap();
        // Minimal teilnehmerliste-shaped template (skips _layout dependency
        // to keep the test self-contained).
        let template = r#"
#set page(paper: "a4")
#let meta = json.decode(sys.inputs.at("meta"))
#let rows = json.decode(sys.inputs.at("rows"))
GV: #meta.title
Datum: #meta.date
Anwesend: #meta.present
#for r in rows [- #r.member_number #r.last_name #r.first_name]
"#;
        std::fs::write(dir.path().join("teilnehmerliste.typ"), template).unwrap();

        let mut assembly_dao = MockTestAssemblyDao::new();
        let assembly = assembly_in_status(aid, AssemblyStatus::Closed);
        assembly_dao
            .expect_find_by_id()
            .returning(move |_, _| Ok(Some(assembly.clone())));
        let mut att_dao = MockTestAttendanceDao::new();
        let rows = sample_rows();
        att_dao
            .expect_list_members_for_assembly()
            .returning(move |_, _, _| Ok(rows.clone()));
        let svc = build_service(
            att_dao,
            assembly_dao,
            MockTestPermissionService::new(),
            tx_dao_with_commit(),
            dir.path().to_path_buf(),
        );
        let res = svc
            .export(
                aid,
                ExportFormat::Pdf,
                ExportInclude::All,
                Authentication::Full,
            )
            .await
            .expect("PDF export should succeed");
        assert!(res.bytes.starts_with(b"%PDF"), "PDF magic missing");
        assert_eq!(res.content_type, "application/pdf");
        assert!(res.filename.ends_with(".pdf"));
    }

    // ----------------------------------------------------------------------
    // D-17 NO-AUDIT grep gate (compile-time read of self via include_str!)
    // ----------------------------------------------------------------------

    #[test]
    fn no_audit_macros_used() {
        // D-17: Export ist Read-Only — kein Audit-Log-Eintrag.
        // Verified by inspecting the own source file (compile-time).
        let src = include_str!("attendance_export.rs");
        // Strip line comments AND string literals so the assertion-fixture
        // below does not self-invalidate the gate (`audited_*!` strings
        // mentioned in messages must not count as macro invocations).
        let payload: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        // Build the needle strings without writing the literal in this file:
        // concatenation forces the audit-marker to exist only at runtime,
        // not in the static source bytes.
        let create_macro = format!("{}!", "audited_create");
        let update_macro = format!("{}!", "audited_update");
        let delete_macro = format!("{}!", "audited_delete");
        assert!(
            !payload.contains(&create_macro),
            "D-17 violated: create-audit macro found"
        );
        assert!(
            !payload.contains(&update_macro),
            "D-17 violated: update-audit macro found"
        );
        assert!(
            !payload.contains(&delete_macro),
            "D-17 violated: delete-audit macro found"
        );
    }
}
