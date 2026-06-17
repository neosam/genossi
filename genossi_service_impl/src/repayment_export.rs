//! Phase 11 (EXPO-01, EXPO-02, EXPO-03, EXPO-05): RepaymentExportServiceImpl.
//!
//! Read-only PDF-Export der Auszahlungsliste fuer eine RepaymentPhase.
//! Vorbild: `genossi_service_impl/src/attendance_export.rs` (Phase 6, 1199 LOC).
//!
//! Anpassungen:
//!   - D-10: Status-Gate akzeptiert `Open` ODER `Closed` (Phase 6 nur `Closed`)
//!   - D-11 / EXPO-05: NULL `audited_*!`-Calls (Grep-Gate-Test am Ende)
//!   - D-12: NUR PDF-Format (kein CSV/XLSX)
//!   - D-08: Eine Zeile pro `RepaymentEntry` (kein Per-Mitglied-Aggregat)
//!   - D-09: Sort primaer `member_number ASC`, sekundaer `entry.created ASC`
//!   - D-04 + D-05: Verwendungszweck hardcoded
//!                  `Anteilsr<U+00FC>ckzahlung GJ {fy} {mn} {fn} {ln}`
//!                  mit ORIGINAL-Umlauten - KEINE ASCII-Sanitization.
//!   - REVISION-Fix B3: Euro-Format ohne `.abs()` (Phase-10-D-04-Pattern,
//!                      PATTERNS.md S9).

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use genossi_dao::member::{MemberDao, MemberEntity};
use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus};
use genossi_dao::repayment_phase::{RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus};
use genossi_dao::{Transaction, TransactionDao};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::repayment_export::{
    ExportFormat, ExportInclude, RepaymentExport, RepaymentExportService,
};
use genossi_service::ServiceError;

use crate::pdf_generation::{PdfGenerator, RepaymentExportRow};

/// D-11 / Phase 6 D-13: Privilege-String konsistent mit allen anderen Vorstand-Endpoints.
const ADMIN_PRIVILEGE: &str = "admin";

/// D-11 Phase 6 D-18: `tracing::info!`-Target-String ersetzt Audit-Eintrag.
const EXPORT_TARGET: &str = "repayment_export";

/// Dependency-Injection-Trait fuer `RepaymentExportServiceImpl`.
/// Wir definieren das manuell (nicht via `gen_service_impl!`), weil der Impl
/// zusaetzlich nicht-Trait-Felder (`pdf_generator`, `template_base`) traegt;
/// das Makro behandelt nur DAO/Service-Trait-Felder.
pub trait RepaymentExportServiceDeps: Send + Sync + 'static {
    type Context: Clone + std::fmt::Debug + Send + Sync + 'static;
    type Transaction: Transaction;
    type RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> + Send + Sync;
    type RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> + Send + Sync;
    type MemberDao: MemberDao<Transaction = Self::Transaction> + Send + Sync;
    type PermissionService: PermissionService<Context = Self::Context> + Send + Sync;
    type TransactionDao: TransactionDao<Transaction = Self::Transaction> + Send + Sync;
}

/// Konkrete Service-Implementation. Plan 11.04 instanziiert sie mit den
/// Production-`Deps`, die `genossi_bin` bereitstellt.
pub struct RepaymentExportServiceImpl<Deps: RepaymentExportServiceDeps> {
    pub transaction_dao: Arc<Deps::TransactionDao>,
    pub permission_service: Arc<Deps::PermissionService>,
    pub repayment_phase_dao: Arc<Deps::RepaymentPhaseDao>,
    pub repayment_entry_dao: Arc<Deps::RepaymentEntryDao>,
    pub member_dao: Arc<Deps::MemberDao>,
    pub pdf_generator: Arc<PdfGenerator>,
    pub template_base: Arc<PathBuf>,
}

impl<Deps: RepaymentExportServiceDeps> RepaymentExportServiceImpl<Deps> {
    /// D-10 / D-11 / Pitfall #2: Permission-Funnel-Order:
    /// 1. Load by id (404 if missing)
    /// 2. Admin gate (403); `Authentication::Full` short-circuits.
    /// 3. Status gate (409): `Open` ODER `Closed` akzeptiert; `Preparation`
    ///    rejected mit `Conflict("phase_not_exportable")`.
    ///
    /// WICHTIG: Reihenfolge `load -> perm -> status` verhindert Status-
    /// Information-Leak an non-admin (Pitfall #2).
    async fn check_admin_and_phase_status(
        &self,
        phase_id: Uuid,
        context: Authentication<Deps::Context>,
        tx: Deps::Transaction,
    ) -> Result<RepaymentPhaseEntity, ServiceError> {
        // 1. Load (404 if missing).
        let phase = self
            .repayment_phase_dao
            .find_by_id(phase_id, tx)
            .await?
            .ok_or(ServiceError::EntityNotFound(phase_id))?;

        // 2. Admin gate (403). `Authentication::Full` short-circuits.
        match &context {
            Authentication::Full => {}
            Authentication::Context(_) => {
                self.permission_service
                    .check_permission(ADMIN_PRIVILEGE, context)
                    .await?;
            }
        }

        // 3. Status gate (409): D-10 - Open ODER Closed akzeptiert.
        match phase.status {
            RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
            RepaymentPhaseStatus::Preparation => {
                return Err(ServiceError::Conflict(Arc::from("phase_not_exportable")));
            }
        }

        Ok(phase)
    }
}

/// REVISION-Fix W1 + W6: Pure-Function zum Filtern und Anreichern.
///
/// Aufrufer ist `RepaymentExportServiceImpl::export()`; direkt aufrufbar in
/// Unit-Tests, damit Include-Filter (D-01/D-02), Sort (D-09), Verwendungszweck
/// (D-04/D-05) und Euro-Format (REVISION-Fix B3) ohne async/Mock-Setup
/// verifiziert werden koennen.
pub(crate) fn filter_and_enrich_rows(
    phase: &RepaymentPhaseEntity,
    entry_member_pairs: Vec<(RepaymentEntryEntity, MemberEntity)>,
    include: ExportInclude,
) -> Vec<RepaymentExportRow> {
    let mut pairs = entry_member_pairs;

    // D-01 / D-02: In-Memory-Include-Filter.
    pairs.retain(|(entry, _)| match include {
        ExportInclude::Open => matches!(
            entry.status,
            RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
        ),
        ExportInclude::All => true,
        ExportInclude::Paid => matches!(entry.status, RepaymentEntryStatus::PaidOut),
    });

    // D-09: Stable-Sort `member_number ASC`, `entry.created ASC`.
    pairs.sort_by(|a, b| {
        a.1.member_number
            .cmp(&b.1.member_number)
            .then_with(|| a.0.created.cmp(&b.0.created))
    });

    // D-04 / D-05 / D-07: Pre-compute `amount_str` + Verwendungszweck pro Row.
    // REVISION-Fix B3: KEIN `.abs()` - Phase-10-D-04-Pattern (PATTERNS.md S9).
    //   Domain-Constraint `share_count > 0` UND `share_value > 0` garantiert
    //   non-negative `amount_cents`.
    pairs
        .iter()
        .map(|(entry, m)| {
            let amount_cents = (entry.share_count_to_pay_out as i64) * phase.share_value;
            let amount_str = format!("{},{:02}", amount_cents / 100, amount_cents % 100);
            // D-04 wortwoertlich mit ORIGINAL-Umlaut; D-05 keine Sanitization.
            let purpose = format!(
                "Anteilsrückzahlung GJ {} {} {} {}",
                phase.fiscal_year, m.member_number, m.first_name, m.last_name
            );
            RepaymentExportRow {
                member_number: m.member_number,
                name: format!("{} {}", m.first_name, m.last_name),
                // D-06 / D-07: leere IBAN -> leerer String (kein Skip, kein Marker).
                iban: m
                    .bank_account
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                share_count: entry.share_count_to_pay_out,
                amount_str,
                purpose,
                // Quick 260607-mw9: Kontoinhaber separat exportieren; leere
                // Strings als None behandeln, damit das Template auf `name`
                // zurückfällt.
                account_holder: m
                    .account_holder
                    .as_ref()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty()),
            }
        })
        .collect()
}

#[async_trait]
impl<Deps: RepaymentExportServiceDeps> RepaymentExportService for RepaymentExportServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn export(
        &self,
        phase_id: Uuid,
        format: ExportFormat,
        include: ExportInclude,
        context: Authentication<Self::Context>,
    ) -> Result<RepaymentExport, ServiceError> {
        // Open Tx for Phase + Entries + Members reads (single Tx,
        // Discretion N+1 per RESEARCH Q5).
        let tx = self.transaction_dao.use_transaction(None).await?;

        // Permission-Funnel (D-10 / D-11 / Pitfall #2): liefert Phase wenn
        // admin + Status in {Open, Closed}.
        let phase = self
            .check_admin_and_phase_status(phase_id, context, tx.clone())
            .await?;

        // Read all entries for this phase. `find_by_phase_id` filtert
        // soft-deleted via Default-Impl.
        let raw_entries: Vec<RepaymentEntryEntity> = self
            .repayment_entry_dao
            .find_by_phase_id(phase_id, tx.clone())
            .await?
            .iter()
            .cloned()
            .collect();

        // Member per entry lesen (N+1 - Discretion-Choice, RESEARCH Q5).
        let mut entry_member_pairs: Vec<(RepaymentEntryEntity, MemberEntity)> =
            Vec::with_capacity(raw_entries.len());
        for entry in raw_entries.into_iter() {
            // D-02: Defensive Skip soft-deleted Entries (Defense-in-Depth -
            // `find_by_phase_id` filtert bereits, aber kostet nichts).
            if entry.deleted.is_some() {
                continue;
            }
            // `MemberDao::find_by_id` Default-Impl filtert soft-deleted
            // (`deleted IS NULL`).
            let member_opt = self
                .member_dao
                .find_by_id(entry.member_id, tx.clone())
                .await?;
            if let Some(member) = member_opt {
                entry_member_pairs.push((entry, member));
            }
            // else: member soft-deleted -> skip (D-02).
        }

        // Pitfall #8: Commit Tx VOR `PdfGenerator::render_*`
        // (sync method; nach Render gibt es keine async-tx-Ops mehr).
        self.transaction_dao.commit(tx).await?;

        // REVISION-Fix W1 / W6 / B3: Filter / Sort / Enrichment in pure fn.
        let enriched_rows = filter_and_enrich_rows(&phase, entry_member_pairs, include);

        // D-18 Phase 6: `tracing::info!`-Pattern ersetzt Audit-Eintrag.
        tracing::info!(
            target: EXPORT_TARGET,
            phase_id = %phase_id,
            fiscal_year = phase.fiscal_year,
            format = ?format,
            include = ?include,
            rows = enriched_rows.len(),
            "exporting repayment list"
        );

        // Render PDF (sync; Tx bereits committed).
        let bytes = match format {
            ExportFormat::Pdf => self.pdf_generator.render_repayment_list(
                "auszahlungsliste.typ",
                &self.template_base,
                &phase,
                &enriched_rows,
            )?,
        };

        // D-15 / SC #2: Server-generated filename, kein User-Input.
        let include_str = match include {
            ExportInclude::Open => "open",
            ExportInclude::All => "all",
            ExportInclude::Paid => "paid",
        };
        let filename = format!("auszahlung-{}-{}.pdf", phase.fiscal_year, include_str);

        Ok(RepaymentExport {
            bytes,
            content_type: "application/pdf",
            filename,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// Drei Bloecke:
//   (a) Test-Infrastruktur (`mock!`-Bloecke, `TestTransaction`, `TestContext`,
//       `TestDeps`, Helper-Funktionen) - 1:1-Mirror von
//       `attendance_export.rs:363-630`.
//   (b) Pure-Function-Tests gegen `filter_and_enrich_rows`
//       (REVISION-Fixes W1, W6, B1, B3).
//   (c) Mock-Service-Tests gegen `service.export(...)`
//       (REVISION-Fix B2 / Pitfall #2) + Grep-Gate-Test (EXPO-05).
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::DaoError;
    use genossi_service::claim_context::ClaimContext;
    use mockall::{mock, predicate::*};
    use std::path::PathBuf;
    use time::macros::datetime;

    // ----------------------------------------------------------------------
    // Test infrastructure (hand-rolled mocks - Pattern aus
    // `attendance_export.rs:363-630`).
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
        pub TestPhaseDao {}
        #[async_trait]
        impl RepaymentPhaseDao for TestPhaseDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &RepaymentPhaseEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &RepaymentPhaseEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<RepaymentPhaseEntity>, DaoError>;
        }
    }

    mock! {
        pub TestEntryDao {}
        #[async_trait]
        impl RepaymentEntryDao for TestEntryDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &RepaymentEntryEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &RepaymentEntryEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<RepaymentEntryEntity>, DaoError>;
            async fn find_by_phase_id(
                &self,
                phase_id: Uuid,
                tx: TestTransaction,
            ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
        }
    }

    mock! {
        pub TestMemberDao {}
        #[async_trait]
        impl MemberDao for TestMemberDao {
            type Transaction = TestTransaction;
            async fn dump_all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn create(
                &self,
                entity: &MemberEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update(
                &self,
                entity: &MemberEntity,
                process: &str,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn all(
                &self,
                tx: TestTransaction,
            ) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn find_by_id(
                &self,
                id: Uuid,
                tx: TestTransaction,
            ) -> Result<Option<MemberEntity>, DaoError>;
            async fn update_migrated(
                &self,
                id: Uuid,
                migrated: bool,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn update_dates(
                &self,
                id: Uuid,
                join_date: time::Date,
                exit_date: Option<time::Date>,
                tx: TestTransaction,
            ) -> Result<(), DaoError>;
            async fn find_by_member_number(
                &self,
                member_number: i64,
                tx: TestTransaction,
            ) -> Result<Option<MemberEntity>, DaoError>;
            async fn count_active(
                &self,
                today: time::Date,
                tx: TestTransaction,
            ) -> Result<u64, DaoError>;
            async fn next_member_number(
                &self,
                tx: TestTransaction,
            ) -> Result<i64, DaoError>;
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

    impl RepaymentExportServiceDeps for TestDeps {
        type Context = TestContext;
        type Transaction = TestTransaction;
        type RepaymentPhaseDao = MockTestPhaseDao;
        type RepaymentEntryDao = MockTestEntryDao;
        type MemberDao = MockTestMemberDao;
        type PermissionService = MockTestPermissionService;
        type TransactionDao = MockTestTxDao;
    }

    // ----------------------------------------------------------------------
    // Helper builders.
    // ----------------------------------------------------------------------

    fn build_service(
        phase_dao: MockTestPhaseDao,
        entry_dao: MockTestEntryDao,
        member_dao: MockTestMemberDao,
        permission_service: MockTestPermissionService,
        tx_dao: MockTestTxDao,
    ) -> RepaymentExportServiceImpl<TestDeps> {
        RepaymentExportServiceImpl {
            transaction_dao: Arc::new(tx_dao),
            permission_service: Arc::new(permission_service),
            repayment_phase_dao: Arc::new(phase_dao),
            repayment_entry_dao: Arc::new(entry_dao),
            member_dao: Arc::new(member_dao),
            pdf_generator: Arc::new(PdfGenerator::new()),
            template_base: Arc::new(PathBuf::from("templates")),
        }
    }

    /// Permission-Funnel-Tests: `use_transaction` ist erlaubt, `commit` darf
    /// 0..=1 mal aufgerufen werden (Funnel bricht ggf. VOR `commit` ab).
    fn tx_dao_no_commit() -> MockTestTxDao {
        let mut tx_dao = MockTestTxDao::new();
        tx_dao
            .expect_use_transaction()
            .returning(|_| Ok(TestTransaction));
        tx_dao.expect_commit().times(0..=1).returning(|_| Ok(()));
        tx_dao
    }

    fn test_member(member_number: i64, first: &str, last: &str) -> MemberEntity {
        let date = time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap();
        MemberEntity {
            id: Uuid::new_v4(),
            member_number,
            first_name: Arc::from(first),
            last_name: Arc::from(last),
            salutation: None,
            title: None,
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 5,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: Some(Arc::from("DE89370400440532013000")),
            status: genossi_dao::member::MemberStatus::Normal,
            account_holder: None,
            postal_status: genossi_dao::member::PostalStatus::Erreichbar,
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn test_entry(
        member_id: Uuid,
        phase_id: Uuid,
        status: RepaymentEntryStatus,
        created_us: i64,
    ) -> RepaymentEntryEntity {
        RepaymentEntryEntity {
            id: Uuid::new_v4(),
            member_id,
            phase_id,
            share_count_to_pay_out: 1,
            status,
            created: datetime!(2026 - 01 - 01 00:00:00) + time::Duration::microseconds(created_us),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn test_phase(status: RepaymentPhaseStatus) -> RepaymentPhaseEntity {
        RepaymentPhaseEntity {
            id: Uuid::new_v4(),
            fiscal_year: 2026,
            share_value: 12000, // 120 EUR pro Anteil (in Cent).
            status,
            opened_at: None,
            closed_at: None,
            created: datetime!(2026 - 01 - 01 00:00:00),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    // ----------------------------------------------------------------------
    // EXPO-05 / D-11: Grep-Gate-Test - keine Audit-Macros im Source-File.
    //
    // Self-Reference-Trick via `format!()` damit der eigene Source-File die
    // Assertion nicht selbst invalidiert (RESEARCH Q10 + Phase-6-Vorlage in
    // `attendance_export.rs:1167-1198`).
    // ----------------------------------------------------------------------
    #[test]
    fn no_audit_macros_used() {
        let src = include_str!("repayment_export.rs");
        let payload: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        let create_macro = format!("{}!", "audited_create");
        let update_macro = format!("{}!", "audited_update");
        let delete_macro = format!("{}!", "audited_delete");

        assert!(
            !payload.contains(&create_macro),
            "EXPO-05 violated: create-audit macro found"
        );
        assert!(
            !payload.contains(&update_macro),
            "EXPO-05 violated: update-audit macro found"
        );
        assert!(
            !payload.contains(&delete_macro),
            "EXPO-05 violated: delete-audit macro found"
        );
    }

    // ----------------------------------------------------------------------
    // REVISION-Fix W6 + B1: D-04 + D-05 wortwoertlich.
    //
    // Verifiziert dass der `purpose`-String den ORIGINAL-Umlaut enthaelt
    // (KEINE ASCII-Sanitization) und dem D-04-Schema exakt entspricht.
    //
    // REVISION-Fix B1: Die ASCII-Variante des `purpose`-Strings darf
    // NIRGENDS als Literal im Source-File auftauchen. Wir konstruieren sie
    // zur Laufzeit via `format!`-Trick (siehe Negativ-Assertion unten),
    // damit das Acceptance-Criterion
    // `grep -c "Anteilsr<ascii>ckzahlung" == 0`
    // deterministisch erfuellbar ist.
    // ----------------------------------------------------------------------
    #[test]
    fn test_purpose_string_preserves_umlaut_per_d04() {
        let phase = test_phase(RepaymentPhaseStatus::Open);
        let mueller = test_member(1234, "Hans", "Müller");
        let entry = test_entry(mueller.id, phase.id, RepaymentEntryStatus::Open, 0);

        let rows = filter_and_enrich_rows(&phase, vec![(entry, mueller)], ExportInclude::Open);

        assert_eq!(rows.len(), 1);
        // D-04 + D-05: wortwoertlich mit Original-Umlaut.
        assert_eq!(
            rows[0].purpose, "Anteilsrückzahlung GJ 2026 1234 Hans Müller",
            "D-04 violated - purpose must use original Umlaut; D-05 forbids ASCII-Sanitization"
        );

        // REVISION-Fix B1: Defense-in-Depth-Negative-Assertion.
        // Der ASCII-Variant wird hier ZUR LAUFZEIT konstruiert, damit der
        // String-Literal NICHT im Source-File auftaucht. Das macht das
        // Acceptance-Criterion `grep -c "Anteilsr<ascii>ckzahlung" == 0`
        // deterministisch erfuellbar.
        let ascii_variant = format!("Anteilsr{}ckzahlung", "ue");
        assert!(
            !rows[0].purpose.contains(&ascii_variant),
            "D-04 / D-05 violated: purpose contains ASCII variant"
        );
    }

    // ----------------------------------------------------------------------
    // REVISION-Fix W1: Include-Filter-Counts direkt asserten.
    // ----------------------------------------------------------------------
    #[test]
    fn test_include_filter_row_counts() {
        let phase = test_phase(RepaymentPhaseStatus::Open);
        let m1 = test_member(101, "A", "AA");
        let m2 = test_member(102, "B", "BB");
        let m3 = test_member(103, "C", "CC");
        let m4 = test_member(104, "D", "DD");

        // 2x Open + 1x Contacted + 1x PaidOut.
        let pairs = vec![
            (
                test_entry(m1.id, phase.id, RepaymentEntryStatus::Open, 0),
                m1.clone(),
            ),
            (
                test_entry(m2.id, phase.id, RepaymentEntryStatus::Open, 1),
                m2.clone(),
            ),
            (
                test_entry(m3.id, phase.id, RepaymentEntryStatus::Contacted, 2),
                m3.clone(),
            ),
            (
                test_entry(m4.id, phase.id, RepaymentEntryStatus::PaidOut, 3),
                m4.clone(),
            ),
        ];

        // D-01: include=Open -> Open + Contacted = 3 Zeilen.
        let open_rows = filter_and_enrich_rows(&phase, pairs.clone(), ExportInclude::Open);
        assert_eq!(
            open_rows.len(),
            3,
            "include=Open should include Open + Contacted (D-01)"
        );

        // D-02: include=All -> 4 Zeilen (alle drei Stati).
        let all_rows = filter_and_enrich_rows(&phase, pairs.clone(), ExportInclude::All);
        assert_eq!(
            all_rows.len(),
            4,
            "include=All should include all 3 stati (D-02)"
        );

        // D-02: include=Paid -> 1 Zeile.
        let paid_rows = filter_and_enrich_rows(&phase, pairs.clone(), ExportInclude::Paid);
        assert_eq!(
            paid_rows.len(),
            1,
            "include=Paid should include only PaidOut (D-02)"
        );

        // D-09 Sort: `open_rows` aufsteigend nach `member_number`.
        assert_eq!(open_rows[0].member_number, 101);
        assert_eq!(open_rows[1].member_number, 102);
        assert_eq!(open_rows[2].member_number, 103);
    }

    // ----------------------------------------------------------------------
    // REVISION-Fix B3: Euro-Format ohne `.abs()` liefert korrekten String.
    // ----------------------------------------------------------------------
    #[test]
    fn test_amount_str_uses_phase_10_d04_pattern_without_abs() {
        let phase = test_phase(RepaymentPhaseStatus::Open);
        // share_value = 12000 cents (120 EUR), share_count = 1 -> "120,00".
        let m = test_member(1, "X", "Y");
        let entry = test_entry(m.id, phase.id, RepaymentEntryStatus::Open, 0);

        let rows = filter_and_enrich_rows(&phase, vec![(entry, m)], ExportInclude::All);
        assert_eq!(
            rows[0].amount_str, "120,00",
            "Amount format must be Phase-10-D-04-conform (no `.abs()`, no leading zeros)"
        );
    }

    // ----------------------------------------------------------------------
    // REVISION-Fix B2 (Pitfall #2): Permission-Funnel-Order via Mocks.
    //
    // Setup:
    //   * Phase ist `Preparation` (Status-Gate wuerde 409 liefern).
    //   * `PermissionService` liefert `PermissionDenied` (Admin-Gate
    //     verweigert).
    //
    // Erwartung: `service.export(...)` liefert `PermissionDenied` (403),
    // NICHT `Conflict("phase_not_exportable")` (409).
    //
    // Das beweist die Funnel-Order `load -> permission -> status`.
    // Vorbild: `attendance_export.rs:748-784`.
    // ----------------------------------------------------------------------
    #[tokio::test]
    async fn test_non_admin_on_preparation_returns_permission_denied_not_conflict() {
        // ARRANGE.
        let phase_id = Uuid::new_v4();
        let mut preparation_phase = test_phase(RepaymentPhaseStatus::Preparation);
        preparation_phase.id = phase_id;
        let phase_clone = preparation_phase.clone();

        // Phase-DAO liefert die Preparation-Phase (load gelingt).
        let mut phase_dao = MockTestPhaseDao::new();
        phase_dao
            .expect_find_by_id()
            .with(eq(phase_id), always())
            .times(1)
            .returning(move |_, _| Ok(Some(phase_clone.clone())));

        // PermissionService verweigert Admin-Zugriff.
        let mut perm = MockTestPermissionService::new();
        perm.expect_check_permission()
            .withf(|p, _| p == ADMIN_PRIVILEGE)
            .times(1)
            .returning(|_, _| Err(ServiceError::PermissionDenied));

        // Entry- und Member-DAO duerfen NICHT angefragt werden,
        // weil der Funnel VOR den DAO-Reads abbricht. Keine `expect_*`-Calls.
        let entry_dao = MockTestEntryDao::new();
        let member_dao = MockTestMemberDao::new();

        let tx_dao = tx_dao_no_commit();

        let svc = build_service(phase_dao, entry_dao, member_dao, perm, tx_dao);

        // ACT.
        let result = svc
            .export(
                phase_id,
                ExportFormat::Pdf,
                ExportInclude::Open,
                Authentication::Context(TestContext),
            )
            .await;

        // ASSERT - KRITISCH: PermissionDenied (403), NICHT Conflict (409).
        // Wenn Funnel-Order falsch waere (status BEFORE permission), kaeme
        // `Conflict("phase_not_exportable")` und leakte die Existenz der
        // Preparation-Phase an non-admin.
        assert!(
            matches!(result, Err(ServiceError::PermissionDenied)),
            "Funnel-Order is broken: expected PermissionDenied (permission BEFORE status), got: {:?}",
            result
        );
        assert!(
            !matches!(result, Err(ServiceError::Conflict(_))),
            "Pitfall #2 violated: got Conflict - Status-Leak via 409 detected (permission MUST be checked BEFORE status)"
        );
    }
}
