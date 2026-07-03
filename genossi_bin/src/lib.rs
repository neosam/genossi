use std::sync::Arc;

use genossi_dao_impl_sqlite::{TransactionDaoImpl, TransactionImpl};
#[cfg(feature = "oidc")]
use genossi_service::auth_types::AuthenticatedContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::permission::MockContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::user_service::MockUserService;
use genossi_service_impl::application::ApplicationServiceDeps;
use genossi_service_impl::application_document::ApplicationDocumentServiceDeps;
use genossi_service_impl::member::MemberServiceDeps;
use genossi_service_impl::member_action::MemberActionServiceDeps;
use genossi_service_impl::member_document::MemberDocumentServiceDeps;
use genossi_service_impl::member_import::MemberImportServiceDeps;
use genossi_service_impl::permission::PermissionServiceDeps;
use genossi_service_impl::user_preference::UserPreferenceServiceDeps;
use genossi_service_impl::validation::ValidationServiceDeps;
use sqlx::SqlitePool;
use uuid::Uuid as UuidType;

pub struct PoolMemberResolver {
    pool: Arc<SqlitePool>,
}

impl PoolMemberResolver {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl genossi_mail::template::MemberResolver for PoolMemberResolver {
    async fn find_member_by_id(
        &self,
        id: UuidType,
    ) -> Result<Option<genossi_dao::member::MemberEntity>, genossi_mail::service::MailServiceError>
    {
        use genossi_dao::member::MemberDao as _;
        use genossi_dao::TransactionDao as _;
        let transaction_dao = TransactionDaoImpl::new(self.pool.clone());
        let member_dao = genossi_dao_impl_sqlite::member::MemberDaoImpl::new(self.pool.clone());
        let tx = transaction_dao.transaction().await.map_err(|e| {
            genossi_mail::service::MailServiceError::DataAccess(Arc::from(format!("{:?}", e)))
        })?;
        member_dao.find_by_id(id, tx).await.map_err(|e| {
            genossi_mail::service::MailServiceError::DataAccess(Arc::from(format!("{:?}", e)))
        })
    }
}

// Type aliases for clarity
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type Context = MockContext;
#[cfg(feature = "oidc")]
type Context = AuthenticatedContext;
type Transaction = TransactionImpl;
type TransactionDao = TransactionDaoImpl;
type MemberDao = genossi_dao_impl_sqlite::member::MemberDaoImpl;
type PermissionDao = genossi_dao_impl_sqlite::permission::PermissionDaoImpl;
type UuidService = genossi_service_impl::uuid_service::UuidServiceImpl;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type UserService = MockUserService;
#[cfg(feature = "oidc")]
type UserService = genossi_service_impl::user_service::AuthContextUserService;

// Define dependency structures
pub struct PermissionServiceDependencies;

unsafe impl Send for PermissionServiceDependencies {}
unsafe impl Sync for PermissionServiceDependencies {}

impl PermissionServiceDeps for PermissionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type PermissionDao = PermissionDao;
    type UserService = UserService;
}

type PermissionService =
    genossi_service_impl::permission::PermissionServiceImpl<PermissionServiceDependencies>;

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type SessionService = genossi_service_impl::session::MockSessionServiceImpl;

#[cfg(feature = "oidc")]
type SessionService = genossi_service_impl::session::SessionServiceImpl<SessionServiceDependencies>;

#[cfg(feature = "oidc")]
pub struct SessionServiceDependencies;

#[cfg(feature = "oidc")]
unsafe impl Send for SessionServiceDependencies {}

#[cfg(feature = "oidc")]
unsafe impl Sync for SessionServiceDependencies {}

#[cfg(feature = "oidc")]
impl genossi_service_impl::session::SessionServiceDeps for SessionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type PermissionDao = PermissionDao;
    // Plan 02-06: Helper-Session-Discriminator + D-18 status-check needs
    // AssemblyDao + TransactionDao injected into SessionServiceImpl.
    type AssemblyDao = AssemblyDao;
    type TransactionDao = TransactionDao;
}

pub struct MemberServiceDependencies;

unsafe impl Send for MemberServiceDependencies {}
unsafe impl Sync for MemberServiceDependencies {}

impl MemberServiceDeps for MemberServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberService = genossi_service_impl::member::MemberServiceImpl<MemberServiceDependencies>;

type ApplicationDao = genossi_dao_impl_sqlite::application::ApplicationDaoImpl;
// Phase 25 Wave 3: single-slot application-document DAO.
type ApplicationDocumentDao =
    genossi_dao_impl_sqlite::application_document::ApplicationDocumentDaoImpl;
type AssemblyDao = genossi_dao_impl_sqlite::assembly::AssemblyDaoImpl;
type AssemblyMemberSnapshotDao =
    genossi_dao_impl_sqlite::assembly_member_snapshot::AssemblyMemberSnapshotDaoImpl;

pub struct ApplicationServiceDependencies;

unsafe impl Send for ApplicationServiceDependencies {}
unsafe impl Sync for ApplicationServiceDependencies {}

impl ApplicationServiceDeps for ApplicationServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ApplicationDao = ApplicationDao;
    // Phase 25 Wave 3: confirm() carryover to audited MemberDocument.
    type ApplicationDocumentDao = ApplicationDocumentDao;
    type MemberDocumentDao = MemberDocumentDao;
    type DocumentStorage = DocumentStorage;
    type AuditLogDao = AuditLogDao;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
    type ConfigService = ConfigService;
    type MailService = MailServiceType;
}

type ApplicationService =
    genossi_service_impl::application::ApplicationServiceImpl<ApplicationServiceDependencies>;

pub struct AssemblyServiceDependencies;

unsafe impl Send for AssemblyServiceDependencies {}
unsafe impl Sync for AssemblyServiceDependencies {}

impl genossi_service_impl::assembly::AssemblyServiceDeps for AssemblyServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AssemblyDao = AssemblyDao;
    type AssemblyMemberSnapshotDao = AssemblyMemberSnapshotDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
    // Phase 3 Plan 05 cascade additions:
    type HelperTokenDao = HelperTokenDao;
    type PermissionDao = PermissionDao;
}

type AssemblyService =
    genossi_service_impl::assembly::AssemblyServiceImpl<AssemblyServiceDependencies>;

// Phase 7 Plan 04 (D-DI): RepaymentPhaseServiceImpl wiring. Five deps —
// kein Snapshot/MemberDao/HelperTokenDao/PermissionDao (Phase 7 ist simpler
// als Assembly; PATTERNS §10 Minimal-Deps-Liste).
type RepaymentPhaseDao = genossi_dao_impl_sqlite::repayment_phase::RepaymentPhaseDaoImpl;
// Phase 8 Plan 04: RepaymentPhaseServiceImpl is extended with RepaymentEntryDao
// + MemberDao deps so open_phase can auto-fill RepaymentEntries and close_phase
// can validate pending entries in the same transaction.
type RepaymentEntryDao = genossi_dao_impl_sqlite::repayment_entry::RepaymentEntryDaoImpl;

pub struct RepaymentPhaseServiceDependencies;

unsafe impl Send for RepaymentPhaseServiceDependencies {}
unsafe impl Sync for RepaymentPhaseServiceDependencies {}

impl genossi_service_impl::repayment_phase::RepaymentPhaseServiceDeps
    for RepaymentPhaseServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    // Phase 8 Plan 04: open_phase auto-fills RepaymentEntries; close_phase
    // validates pending entries — both need the entry + member DAOs in the
    // same transaction as the status update.
    type RepaymentEntryDao = RepaymentEntryDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type RepaymentPhaseService = genossi_service_impl::repayment_phase::RepaymentPhaseServiceImpl<
    RepaymentPhaseServiceDependencies,
>;

// Phase 8 Plan 05 (D-DI) + Phase 9 (PAYO-01): RepaymentEntryServiceImpl wiring.
// Eight deps — RepaymentEntryDao + RepaymentPhaseDao + MemberDao + MemberActionDao
// (Phase 9: fuer audited_create! MemberAction::Verkauf im mark_paid_out-Cascade) +
// AuditLogDao + PermissionService + UuidService + TransactionDao.
// RepaymentEntryDao, RepaymentPhaseDao werden Arc-shared mit RepaymentPhaseServiceImpl;
// MemberActionDao Arc-shared mit allen 5 bestehenden Konsumenten (W-02:
// exakt 1 DAO-Konstruktor pro Prozess; Phase 9 ist Konsument #6).
pub struct RepaymentEntryServiceDependencies;

unsafe impl Send for RepaymentEntryServiceDependencies {}
unsafe impl Sync for RepaymentEntryServiceDependencies {}

impl genossi_service_impl::repayment_entry::RepaymentEntryServiceDeps
    for RepaymentEntryServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentEntryDao = RepaymentEntryDao;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type RepaymentEntryService = genossi_service_impl::repayment_entry::RepaymentEntryServiceImpl<
    RepaymentEntryServiceDependencies,
>;

type HelperTokenDao = genossi_dao_impl_sqlite::helper_token::HelperTokenDaoImpl;
type AttendanceDao = genossi_dao_impl_sqlite::attendance::AttendanceDaoImpl;

// Phase 3 Plan 06: AttendanceServiceImpl wiring (D-23). Six deps —
// deliberately NO UuidService and NO AuditLogDao (D-08, ATTN-05).
pub struct AttendanceServiceDependencies;

unsafe impl Send for AttendanceServiceDependencies {}
unsafe impl Sync for AttendanceServiceDependencies {}

impl genossi_service_impl::attendance::AttendanceServiceDeps for AttendanceServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AttendanceDao = AttendanceDao;
    type AssemblyDao = AssemblyDao;
    type MemberDao = MemberDao;
    type AssemblyMemberSnapshotDao = AssemblyMemberSnapshotDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type AttendanceService =
    genossi_service_impl::attendance::AttendanceServiceImpl<AttendanceServiceDependencies>;

// Phase 6 Plan 03 (D-13, D-DI): AttendanceExportServiceImpl wiring.
// Four DAO/Service-deps. NO UuidService, NO AuditLogDao (D-17 — export is
// not audited, consistent with ATTN-05). PdfGenerator + template_base are
// non-trait fields constructed inline in RestStateImpl::new() (re-using the
// existing `pdf_generator` + `template_storage` Arcs).
pub struct AttendanceExportServiceDependencies;

unsafe impl Send for AttendanceExportServiceDependencies {}
unsafe impl Sync for AttendanceExportServiceDependencies {}

impl genossi_service_impl::attendance_export::AttendanceExportServiceDeps
    for AttendanceExportServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type AttendanceDao = AttendanceDao;
    type AssemblyDao = AssemblyDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type AttendanceExportService = genossi_service_impl::attendance_export::AttendanceExportServiceImpl<
    AttendanceExportServiceDependencies,
>;

// Phase 11 (EXPO-01..03, EXPO-05): RepaymentExportServiceImpl DI-Aliases.
// Five DAO/Service-deps. KEIN UuidService, KEIN AuditLogDao (D-11 / EXPO-05 —
// Export ist nicht auditiert, konsistent mit AttendanceExport-Pattern). PdfGenerator
// + template_base werden inline in RestStateImpl::new() konstruiert, indem die
// existierenden `pdf_generator` + `template_storage` Arcs wiederverwendet werden
// (Single-Arc-per-Process pro Plan-10-Pattern).
pub struct RepaymentExportServiceDependencies;

unsafe impl Send for RepaymentExportServiceDependencies {}
unsafe impl Sync for RepaymentExportServiceDependencies {}

impl genossi_service_impl::repayment_export::RepaymentExportServiceDeps
    for RepaymentExportServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type RepaymentEntryDao = RepaymentEntryDao;
    type MemberDao = MemberDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type RepaymentExportService = genossi_service_impl::repayment_export::RepaymentExportServiceImpl<
    RepaymentExportServiceDependencies,
>;

// Phase 13 D-13-04 / D-13-10: RepaymentContextResolverImpl — shared aggregation
// helper used by the Letter-Service (and, after a follow-up /gsd-quick, by the
// Phase-10 Mail-Worker; D-13-11 pending todo).
pub struct RepaymentContextResolverDependencies;

unsafe impl Send for RepaymentContextResolverDependencies {}
unsafe impl Sync for RepaymentContextResolverDependencies {}

impl genossi_service_impl::repayment_context::RepaymentContextResolverDeps
    for RepaymentContextResolverDependencies
{
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type RepaymentEntryDao = RepaymentEntryDao;
}

type RepaymentContextResolver =
    genossi_service_impl::repayment_context::RepaymentContextResolverImpl<
        RepaymentContextResolverDependencies,
    >;

// Phase 13 D-13-01..11: RepaymentLetterServiceImpl wiring.
// Ten DAO/Service-deps (5 like RepaymentExport PLUS MemberDocumentDao,
// AuditLogDao, UuidService, DocumentStorage, RepaymentContextResolver — see
// genossi_service_impl/src/repayment_letter.rs::RepaymentLetterServiceDeps).
// PdfGenerator + template_base + document_storage werden inline in
// RestStateImpl::new() konstruiert ueber die existierenden Arcs (Single-Arc-
// per-Process pro Plan-10-Pattern: KEIN neuer Arc::new(...) fuer die DAOs).
pub struct RepaymentLetterServiceDependencies;

unsafe impl Send for RepaymentLetterServiceDependencies {}
unsafe impl Sync for RepaymentLetterServiceDependencies {}

impl genossi_service_impl::repayment_letter::RepaymentLetterServiceDeps
    for RepaymentLetterServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type RepaymentEntryDao = RepaymentEntryDao;
    type MemberDao = MemberDao;
    type MemberDocumentDao = MemberDocumentDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
    type UuidService = UuidService;
    type RepaymentContextResolver = RepaymentContextResolver;
    type DocumentStorage = DocumentStorage;
}

type RepaymentLetterService = genossi_service_impl::repayment_letter::RepaymentLetterServiceImpl<
    RepaymentLetterServiceDependencies,
>;

pub struct HelperTokenServiceDependencies;

unsafe impl Send for HelperTokenServiceDependencies {}
unsafe impl Sync for HelperTokenServiceDependencies {}

impl genossi_service_impl::helper_token::HelperTokenServiceDeps for HelperTokenServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type HelperTokenDao = HelperTokenDao;
    type AssemblyDao = AssemblyDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type PermissionDao = PermissionDao;
    type SessionService = SessionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type HelperTokenService =
    genossi_service_impl::helper_token::HelperTokenServiceImpl<HelperTokenServiceDependencies>;

// Plan 02-07 Task 3: DbAssemblyStatusProbe is the production adapter that
// answers `MockSessionServiceImpl::with_probe(...)`'s `is_open(...)` query
// against the real DB. Only compiled in the mock_auth-build because the
// oidc-build uses `SessionServiceImpl` which embeds the assembly_dao directly.
//
// The probe is best-effort: any DB error (failed transaction, lookup error)
// is treated as "not open" — D-18 cascade-safe (better to reject helper
// cookies than to leak access if the assembly state is unknown).
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
struct DbAssemblyStatusProbe {
    assembly_dao: Arc<AssemblyDao>,
    transaction_dao: Arc<TransactionDao>,
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
#[async_trait::async_trait]
impl genossi_service_impl::session::AssemblyStatusProbe for DbAssemblyStatusProbe {
    async fn is_open(&self, assembly_id: uuid::Uuid) -> bool {
        use genossi_dao::assembly::AssemblyDao as _;
        use genossi_dao::TransactionDao as _;
        let Ok(tx) = self.transaction_dao.use_transaction(None).await else {
            return false;
        };
        let result = self.assembly_dao.find_by_id(assembly_id, tx).await;
        // We acquired our own transaction; commit it (read-only operation,
        // no side-effect to roll back).
        // The transaction was moved into find_by_id; we need a fresh one for
        // the commit, but TransactionDao::commit takes owned tx — so we have
        // to re-acquire. In practice, find_by_id consumes the transaction.
        matches!(
            result,
            Ok(Some(a)) if a.status == genossi_dao::assembly::AssemblyStatus::Open
        )
    }
}

pub struct MemberImportServiceDependencies;

unsafe impl Send for MemberImportServiceDependencies {}
unsafe impl Sync for MemberImportServiceDependencies {}

impl MemberImportServiceDeps for MemberImportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberImportService =
    genossi_service_impl::member_import::MemberImportServiceImpl<MemberImportServiceDependencies>;

type AuditLogDao = genossi_dao_impl_sqlite::audit_log::AuditLogDaoImpl;
type MemberActionDao = genossi_dao_impl_sqlite::member_action::MemberActionDaoImpl;
type MemberDocumentDao = genossi_dao_impl_sqlite::member_document::MemberDocumentDaoImpl;
type BackupDao = genossi_dao_impl_sqlite::backup::BackupDaoImpl;
type BackupDocumentSyncDao = genossi_dao_impl_sqlite::backup::BackupDocumentSyncDaoImpl;
type BackupCommunicationSyncDao = genossi_dao_impl_sqlite::backup::BackupCommunicationSyncDaoImpl;
type AuditTimestampDao = genossi_dao_impl_sqlite::audit_timestamp::AuditTimestampDaoImpl;
type TimestampServiceType = genossi_service_impl::timestamp::TimestampServiceImpl<
    TransactionDao,
    AuditTimestampDao,
    AuditLogDao,
    ConfigService,
>;

pub struct MemberActionServiceDependencies;

unsafe impl Send for MemberActionServiceDependencies {}
unsafe impl Sync for MemberActionServiceDependencies {}

impl MemberActionServiceDeps for MemberActionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberActionDao = MemberActionDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberActionService =
    genossi_service_impl::member_action::MemberActionServiceImpl<MemberActionServiceDependencies>;

// Phase 15 v1.2 (D-15-16): MembershipAdjustService DI-Wiring.
// Selbes Deps-Set wie MemberActionService — Service braucht member_action_dao,
// member_dao, audit_log_dao, permission_service, uuid_service, transaction_dao
// (siehe `genossi_service_impl/src/membership_adjust.rs::gen_service_impl!`).
pub struct MembershipAdjustServiceDependencies;

unsafe impl Send for MembershipAdjustServiceDependencies {}
unsafe impl Sync for MembershipAdjustServiceDependencies {}

impl genossi_service_impl::membership_adjust::MembershipAdjustServiceDeps
    for MembershipAdjustServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type MemberActionDao = MemberActionDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
    // Phase 16 (D-16-02 Inlining + D-16-08 Sum-Check): zwei neue DAO-Deps fuer
    // `partial_repayment` — inlined Phase-Auto-Create via `repayment_phase_dao`,
    // Sum-Check + Entry-Create via `repayment_entry_dao`.
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type RepaymentEntryDao = RepaymentEntryDao;
}

type MembershipAdjustService = genossi_service_impl::membership_adjust::MembershipAdjustServiceImpl<
    MembershipAdjustServiceDependencies,
>;

pub struct MemberDocumentServiceDependencies;

unsafe impl Send for MemberDocumentServiceDependencies {}
unsafe impl Sync for MemberDocumentServiceDependencies {}

impl MemberDocumentServiceDeps for MemberDocumentServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberDocumentDao = MemberDocumentDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberDocumentService = genossi_service_impl::member_document::MemberDocumentServiceImpl<
    MemberDocumentServiceDependencies,
>;

type DocumentStorage = genossi_service_impl::document_storage::FilesystemDocumentStorage;

// Phase 25 Wave 3 (Plan 25-04): ApplicationDocumentServiceImpl DI wiring.
pub struct ApplicationDocumentServiceDependencies;

unsafe impl Send for ApplicationDocumentServiceDependencies {}
unsafe impl Sync for ApplicationDocumentServiceDependencies {}

impl ApplicationDocumentServiceDeps for ApplicationDocumentServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ApplicationDocumentDao = ApplicationDocumentDao;
    type ApplicationDao = ApplicationDao;
    type DocumentStorage = DocumentStorage;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type ApplicationDocumentService =
    genossi_service_impl::application_document::ApplicationDocumentServiceImpl<
        ApplicationDocumentServiceDependencies,
    >;

pub struct ValidationServiceDependencies;

unsafe impl Send for ValidationServiceDependencies {}
unsafe impl Sync for ValidationServiceDependencies {}

impl ValidationServiceDeps for ValidationServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type ValidationService =
    genossi_service_impl::validation::ValidationServiceImpl<ValidationServiceDependencies>;

type UserPreferenceDao = genossi_dao_impl_sqlite::user_preference::UserPreferenceDaoImpl;

pub struct UserPreferenceServiceDependencies;

unsafe impl Send for UserPreferenceServiceDependencies {}
unsafe impl Sync for UserPreferenceServiceDependencies {}

impl UserPreferenceServiceDeps for UserPreferenceServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type UserPreferenceDao = UserPreferenceDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type UserPreferenceService = genossi_service_impl::user_preference::UserPreferenceServiceImpl<
    UserPreferenceServiceDependencies,
>;

type ConfigDao = genossi_config::dao_sqlite::ConfigDaoSqlite;
type ConfigService = genossi_config::service::ConfigServiceImpl<ConfigDao>;
type MailJobDao = genossi_mail::dao_sqlite::MailJobDaoSqlite;
type MailRecipientDao = genossi_mail::dao_sqlite::MailRecipientDaoSqlite;
type MailRecipientAttachmentDao = genossi_mail::dao_sqlite::MailRecipientAttachmentDaoSqlite;
type MailTemplateDaoType = genossi_mail::dao_sqlite::MailTemplateDaoSqlite;
type MailTemplateServiceType =
    genossi_mail::mail_template_service::MailTemplateServiceImpl<MailTemplateDaoType>;
type StaticDocumentDaoType = genossi_mail::dao_sqlite::StaticDocumentDaoSqlite;
type MailJobStaticAttachmentDaoType = genossi_mail::dao_sqlite::MailJobStaticAttachmentDaoSqlite;
type InboundMailDaoType = genossi_mail::dao_sqlite::InboundMailDaoSqlite;
type InboundMailAttachmentDaoType = genossi_mail::dao_sqlite::InboundMailAttachmentDaoSqlite;
// Phase 20 (Plan 02): Digest-Worker persistiert das letzte Versanddatum.
type DigestStateDaoType = genossi_mail::dao_sqlite::DigestStateDaoSqlite;
type InboxImapClientType = genossi_mail::inbox_imap::AsyncImapClient;
type InboxServiceType = genossi_mail::inbox::InboxServiceImpl<
    ConfigService,
    InboundMailDaoType,
    InboxImapClientType,
    MailJobDao,
    MailRecipientDao,
    InboundMailAttachmentDaoType,
    DocumentStorage,
    // Quick 260607-s0s: reply now persists per-recipient + job-level
    // attachments; pull in the same DAO impls used by MailServiceType.
    MailRecipientAttachmentDao,
    MailJobStaticAttachmentDaoType,
    StaticDocumentDaoType,
>;
type MailServiceType = genossi_mail::service::MailServiceImpl<
    ConfigService,
    MailJobDao,
    MailRecipientDao,
    MailRecipientAttachmentDao,
    StaticDocumentDaoType,
    MailJobStaticAttachmentDaoType,
>;
type StaticDocumentServiceType = genossi_mail::static_document_service::StaticDocumentServiceImpl<
    StaticDocumentDaoType,
    DocumentStorage,
>;

// RestStateImpl with all services
#[derive(Clone)]
pub struct RestStateImpl {
    public_stats_cache: std::sync::Arc<genossi_rest::public_stats::PublicStatsCache>,
    member_service: Arc<MemberService>,
    member_import_service: Arc<MemberImportService>,
    member_action_service: Arc<MemberActionService>,
    // Phase 15 v1.2 (D-15-16): MembershipAdjustService — cancel + increase_shares.
    membership_adjust_service: Arc<MembershipAdjustService>,
    member_document_service: Arc<MemberDocumentService>,
    permission_service: Arc<PermissionService>,
    session_service: Arc<SessionService>,
    document_storage: Arc<DocumentStorage>,
    validation_service: Arc<ValidationService>,
    user_preference_service: Arc<UserPreferenceService>,
    template_storage: Arc<genossi_service_impl::template_storage::TemplateStorage>,
    pdf_generator: Arc<genossi_service_impl::pdf_generation::PdfGenerator>,
    config_service: Arc<ConfigService>,
    mail_service: Arc<MailServiceType>,
    mail_template_service: Arc<MailTemplateServiceType>,
    inbox_service: Arc<InboxServiceType>,
    static_document_service: Arc<StaticDocumentServiceType>,
    application_service: Arc<ApplicationService>,
    // Phase 25 Wave 3 (Plan 25-04): single-slot Application-document service.
    application_document_service: Arc<ApplicationDocumentService>,
    assembly_service: Arc<AssemblyService>,
    // Phase 7 Plan 04: RepaymentPhase backend foundation.
    repayment_phase_service: Arc<RepaymentPhaseService>,
    // Phase 8 Plan 05: RepaymentEntry CRUD + Batch-Toggle REST surface.
    repayment_entry_service: Arc<RepaymentEntryService>,
    helper_token_service: Arc<HelperTokenService>,
    // Phase 3 Plan 06: AttendanceServiceImpl exposed to REST handlers via
    // AttendanceRestState (D-23 wiring).
    attendance_service: Arc<AttendanceService>,
    // Phase 6 Plan 03: AttendanceExportServiceImpl exposed to REST handlers via
    // AttendanceExportRestState (D-DI wiring).
    attendance_export_service: Arc<AttendanceExportService>,
    // Phase 11 (EXPO-01..03, EXPO-05): RepaymentExportServiceImpl exposed
    // to REST handlers via RepaymentExportRestState (D-DI wiring).
    repayment_export_service: Arc<RepaymentExportService>,
    // Phase 13 D-13-04 / D-13-10: shared aggregation helper.
    // Same Arc passed to start_mail_worker (Quick 260603-h0r refactor); also
    // cloned into the letter-service below. Single Arc per process.
    repayment_context_resolver: Arc<RepaymentContextResolver>,
    // Phase 13 D-13-01..11: RepaymentLetterServiceImpl exposed to REST
    // handlers via RepaymentLetterRestState. Direct-Download Bulk-PDF +
    // audited MemberDocument persistence per unique member.
    repayment_letter_service: Arc<RepaymentLetterService>,
    audit_log_dao: Arc<AuditLogDao>,
    // Phase 10 D-11: DAOs shared with the mail-worker for repayment-context
    // aggregation + auditable MemberDocument-create. Same Arcs as the audited
    // services use, so the worker contributes to the single per-process audit
    // hash chain. See start_mail_worker() below for the wiring.
    member_document_dao: Arc<MemberDocumentDao>,
    repayment_phase_dao: Arc<RepaymentPhaseDao>,
    repayment_entry_dao: Arc<RepaymentEntryDao>,
    mail_template_dao: Arc<MailTemplateDaoType>,
    // Phase 10 D-11 (Plan 10.07 auto-fix Rule 3): transaction_dao is also
    // required by start_mail_worker. The local Arc in new() is reused here
    // (same Arc as all *ServiceImpl fields that carry transaction_dao).
    transaction_dao: Arc<TransactionDao>,
    timestamp_service: Arc<TimestampServiceType>,
    backup_dao: Arc<BackupDao>,
    // Inbox worker dependencies
    worker_inbox_config_service: Arc<ConfigService>,
    worker_inbox_dao: Arc<InboundMailDaoType>,
    worker_inbox_imap_client: Arc<InboxImapClientType>,
    // Phase 19: inbox worker also needs the attachment DAO + storage to
    // persist attachments after a successful mail-create.
    worker_inbox_attachment_dao: Arc<InboundMailAttachmentDaoType>,
    worker_inbox_storage: Arc<DocumentStorage>,
    // Worker dependencies (kept for spawning the background worker)
    worker_config_service: Arc<ConfigService>,
    worker_job_dao: Arc<MailJobDao>,
    worker_recipient_dao: Arc<MailRecipientDao>,
    worker_attachment_dao: Arc<MailRecipientAttachmentDao>,
    worker_static_attachment_dao: Arc<MailJobStaticAttachmentDaoType>,
    // Backup worker dependencies
    backup_config_service: Arc<ConfigService>,
    backup_sync_dao: Arc<BackupDocumentSyncDao>,
    backup_comm_sync_dao: Arc<BackupCommunicationSyncDao>,
    // Pool for direct document resolution queries
    pool: Arc<SqlitePool>,
}

impl RestStateImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        // Create DAOs
        let transaction_dao = Arc::new(TransactionDao::new(pool.clone()));
        let member_dao = Arc::new(MemberDao::new(pool.clone()));
        let permission_dao = Arc::new(genossi_dao_impl_sqlite::permission::PermissionDaoImpl::new(
            pool.clone(),
        ));

        // Create services
        #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
        let user_service = Arc::new(MockUserService);
        #[cfg(feature = "oidc")]
        let user_service = Arc::new(genossi_service_impl::user_service::AuthContextUserService);
        let uuid_service = Arc::new(UuidService::new());

        let permission_service =
            Arc::new(genossi_service_impl::permission::PermissionServiceImpl {
                permission_dao: permission_dao.clone(),
                user_service,
            });

        let member_action_dao = Arc::new(MemberActionDao::new(pool.clone()));
        let audit_log_dao = Arc::new(AuditLogDao::new(pool.clone()));

        let member_service = Arc::new(genossi_service_impl::member::MemberServiceImpl {
            member_dao: member_dao.clone(),
            member_action_dao: member_action_dao.clone(),
            audit_log_dao: audit_log_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        let member_action_service = Arc::new(
            genossi_service_impl::member_action::MemberActionServiceImpl {
                member_action_dao: member_action_dao.clone(),
                member_dao: member_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );

        // Phase 16 (D-16-02 / D-16-08): RepaymentPhaseDao + RepaymentEntryDao
        // werden hier (frueher als zuvor) deklariert, damit
        // `MembershipAdjustServiceImpl` sie ueber Arc::clone() als Dependency
        // erhaelt. Die downstream-Consumer (RepaymentPhaseServiceImpl,
        // RepaymentEntryServiceImpl und Plan-09-Services) nutzen weiterhin
        // dieselben Arcs via `.clone()` — single Arc per DAO pro Process
        // (T-08-05-04 mitigation).
        let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new(pool.clone()));
        let repayment_entry_dao = Arc::new(RepaymentEntryDao::new(pool.clone()));

        // Phase 15 v1.2 (D-15-16) + Phase 16 (D-16-02 / D-16-08):
        // MembershipAdjustService — Phase 15-Set + 2 neue Phase-16-DAO-Deps.
        // Alle Arcs sind die kanonischen Per-Process-Instances (single Arc
        // shared across services).
        let membership_adjust_service = Arc::new(
            genossi_service_impl::membership_adjust::MembershipAdjustServiceImpl {
                member_action_dao: member_action_dao.clone(),
                member_dao: member_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
                // Phase 16 D-16-02/08 — Inlined phase-create + sum-check via
                // existing find_by_member_and_phase. Arc-shared mit
                // RepaymentPhase/Entry-Services weiter unten.
                repayment_phase_dao: repayment_phase_dao.clone(),
                repayment_entry_dao: repayment_entry_dao.clone(),
            },
        );

        let member_document_dao = Arc::new(MemberDocumentDao::new(pool.clone()));

        let member_document_service = Arc::new(
            genossi_service_impl::member_document::MemberDocumentServiceImpl {
                // Phase 10 D-11: clone instead of move — the same Arc is
                // persisted as a RestStateImpl field below and shared with
                // the mail-worker.
                member_document_dao: member_document_dao.clone(),
                member_dao: member_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );

        let document_storage = Arc::new(DocumentStorage::from_env());

        let validation_service =
            Arc::new(genossi_service_impl::validation::ValidationServiceImpl {
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let user_preference_dao = Arc::new(UserPreferenceDao::new(pool.clone()));

        let user_preference_service = Arc::new(
            genossi_service_impl::user_preference::UserPreferenceServiceImpl {
                user_preference_dao,
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );

        let application_dao = Arc::new(ApplicationDao::new(pool.clone()));

        // Phase 25 Wave 3 (Plan 25-04): single-slot application-document DAO.
        // The same Arc is passed into both ApplicationServiceImpl (for
        // confirm() carryover) and ApplicationDocumentServiceImpl (for the
        // three REST endpoints). Single DAO per process.
        let application_document_dao =
            Arc::new(ApplicationDocumentDao::new(pool.clone()));

        let member_import_service = Arc::new(
            genossi_service_impl::member_import::MemberImportServiceImpl {
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );

        // Plan 02-06 + 02-07: assembly_dao is needed BEFORE session_service is
        // constructed because:
        //   - oidc-build: SessionServiceImpl carries AssemblyDao for the D-18
        //     helper-cascade.
        //   - mock_auth-build: DbAssemblyStatusProbe (Plan 02-07) holds the same
        //     DAO and is wired via MockSessionServiceImpl::with_probe so HLPR-05
        //     cascade works end-to-end in E2E tests (Plan 02-08).
        // The same Arc<AssemblyDao> is later cloned into AssemblyServiceImpl
        // and HelperTokenServiceImpl below — DAOs are stateless wrappers over
        // the SqlitePool, so cloning the Arc is fine.
        let assembly_dao = Arc::new(AssemblyDao::new(pool.clone()));

        // Plan 02-06 Task 2: MockSessionServiceImpl is now a struct with an
        // optional assembly-status-probe. Plan 02-07 wires the production
        // probe via `with_probe(...)` so HLPR-05 cascade behaviour is observable
        // in mock_auth E2E tests (Plan 02-08 Task 2).
        #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
        let session_service = {
            let probe = Arc::new(DbAssemblyStatusProbe {
                assembly_dao: assembly_dao.clone(),
                transaction_dao: transaction_dao.clone(),
            })
                as Arc<dyn genossi_service_impl::session::AssemblyStatusProbe>;
            // Plan 02-08: wire a SessionPersister that writes real
            // session rows; required by the helper-token redeem flow
            // because `helper_token.session_id` has a FK to `session(id)`.
            let persister = Arc::new(genossi_service_impl::session::DaoSessionPersister {
                dao: permission_dao.clone(),
            })
                as Arc<dyn genossi_service_impl::session::SessionPersister>;
            Arc::new(
                genossi_service_impl::session::MockSessionServiceImpl::with_probe_and_persister(
                    probe, persister,
                ),
            )
        };

        // Plan 02-06: SessionServiceImpl now needs AssemblyDao + TransactionDao
        // for the helper-claims discriminator + D-18 status-check.
        #[cfg(feature = "oidc")]
        let session_service = Arc::new(genossi_service_impl::session::SessionServiceImpl {
            permission_dao: permission_dao.clone(),
            assembly_dao: assembly_dao.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        let template_storage =
            Arc::new(genossi_service_impl::template_storage::TemplateStorage::from_env());
        let pdf_generator = Arc::new(genossi_service_impl::pdf_generation::PdfGenerator::new());

        let config_dao = ConfigDao::new(pool.clone());
        let config_service = Arc::new(ConfigService::new(config_dao));

        let mail_job_dao = MailJobDao::new(pool.clone());
        let mail_recipient_dao = MailRecipientDao::new(pool.clone());
        let mail_attachment_dao = MailRecipientAttachmentDao::new(pool.clone());
        let mail_static_dao = StaticDocumentDaoType::new(pool.clone());
        let mail_job_static_attachment_dao = MailJobStaticAttachmentDaoType::new(pool.clone());
        let config_dao_for_mail = ConfigDao::new(pool.clone());
        let config_service_for_mail = ConfigService::new(config_dao_for_mail);
        let mail_service = Arc::new(MailServiceType::new(
            config_service_for_mail,
            mail_job_dao,
            mail_recipient_dao,
            mail_attachment_dao,
            mail_static_dao,
            mail_job_static_attachment_dao,
        ));

        // Application service for public join
        let config_dao_for_app = ConfigDao::new(pool.clone());
        let config_service_for_app = Arc::new(ConfigService::new(config_dao_for_app));
        let application_service =
            Arc::new(genossi_service_impl::application::ApplicationServiceImpl {
                application_dao: application_dao.clone(),
                // Phase 25 Wave 3 (Plan 25-04): confirm() carryover to
                // audited MemberDocument uses these three deps inside the
                // same use_transaction block.
                application_document_dao: application_document_dao.clone(),
                member_document_dao: member_document_dao.clone(),
                document_storage: document_storage.clone(),
                audit_log_dao: audit_log_dao.clone(),
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
                config_service: config_service_for_app,
                mail_service: mail_service.clone(),
            });

        // Phase 25 Wave 3 (Plan 25-04): ApplicationDocumentServiceImpl for
        // the three REST endpoints (upload/download/delete).
        let application_document_service = Arc::new(
            genossi_service_impl::application_document::ApplicationDocumentServiceImpl {
                application_document_dao: application_document_dao.clone(),
                application_dao: application_dao.clone(),
                document_storage: document_storage.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );

        // assembly_dao was already constructed above (before session_service).
        let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
        // Phase 3 Plan 05: helper_token_dao needs to be constructed BEFORE
        // AssemblyServiceImpl so the cascade-discovery dependency can be
        // wired. (HelperTokenServiceImpl below clones the same Arc — there
        // is exactly one HelperTokenDaoImpl instance per process.)
        let helper_token_dao = Arc::new(HelperTokenDao::new(pool.clone()));
        let assembly_service = Arc::new(genossi_service_impl::assembly::AssemblyServiceImpl {
            assembly_dao: assembly_dao.clone(),
            // Phase 3 Plan 06: cloned (not moved) so AttendanceServiceImpl
            // below can share the same Arc.
            assembly_member_snapshot_dao: assembly_member_snapshot_dao.clone(),
            member_dao: member_dao.clone(),
            audit_log_dao: audit_log_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
            // Phase 3 Plan 05 cascade additions:
            helper_token_dao: helper_token_dao.clone(),
            permission_dao: permission_dao.clone(),
        });

        // Phase 7 Plan 04 + Phase 8 Plans 04/05: RepaymentPhase + RepaymentEntry
        // service wiring.
        // - W-02 (Plan 8 Plan 05): RepaymentPhaseDao + RepaymentEntryDao werden
        //   GENAU EINMAL gebaut und via Arc::clone an BEIDE Services geteilt
        //   (RepaymentPhaseServiceImpl für Auto-Fill/Close-Validation,
        //   RepaymentEntryServiceImpl für CRUD + Batch-Toggle). Sicherstellt
        //   konsistente DAO-State über alle Aufrufe (T-08-05-04 mitigation).
        // - audit_log_dao wird mit allen anderen audited Services geteilt
        //   (T-07-04-05 mitigation: single hash chain across the workspace).
        //
        // Phase 16 (D-16-02 / D-16-08): repayment_phase_dao und
        // repayment_entry_dao werden bereits weiter oben fuer
        // MembershipAdjustServiceImpl deklariert; hier nur Re-Cloning der
        // bestehenden Arcs (single Arc per DAO pro Process).
        let repayment_phase_service = Arc::new(
            genossi_service_impl::repayment_phase::RepaymentPhaseServiceImpl {
                repayment_phase_dao: repayment_phase_dao.clone(),
                repayment_entry_dao: repayment_entry_dao.clone(),
                member_dao: member_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );
        // Phase 8 Plan 05 + Phase 9: RepaymentEntryServiceImpl with 8 deps.
        // Uses the SAME repayment_phase_dao + repayment_entry_dao + member_dao
        // + member_action_dao (Phase 9) Arcs as other services — single DAO
        // instance per process per W-02. member_action_dao ist Konsument #6
        // (bereits geteilt mit MemberService, MemberActionService, ValidationService,
        // MemberImportService, ApplicationService — siehe Z. 568, 577, 604, 625, 709).
        let repayment_entry_service = Arc::new(
            genossi_service_impl::repayment_entry::RepaymentEntryServiceImpl {
                repayment_entry_dao: repayment_entry_dao.clone(),
                repayment_phase_dao: repayment_phase_dao.clone(),
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );

        // Phase 3 Plan 06 (D-23): AttendanceServiceImpl with 6 deps —
        // AttendanceDao, AssemblyDao, MemberDao, AssemblyMemberSnapshotDao,
        // PermissionService, TransactionDao. No UuidService, no AuditLogDao
        // (D-08, ATTN-05 — attendance is not audited).
        let attendance_dao = Arc::new(AttendanceDao::new(pool.clone()));
        let attendance_service =
            Arc::new(genossi_service_impl::attendance::AttendanceServiceImpl {
                // Phase 6 Plan 03: cloned (not moved) so AttendanceExportServiceImpl
                // below can share the same Arc.
                attendance_dao: attendance_dao.clone(),
                assembly_dao: assembly_dao.clone(),
                member_dao: member_dao.clone(),
                assembly_member_snapshot_dao,
                permission_service: permission_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        // Phase 6 Plan 03: AttendanceExportServiceImpl (D-01..D-18 backend).
        // Re-uses the existing `pdf_generator` (line 585) and
        // `template_storage` (line 583) Arcs — KEINE neue PdfGenerator::new()-
        // Instanz und KEINE neue PathBuf::from("templates")-Literal, sonst
        // wuerde sich der Export-Pfad vom MemberDocument/Application-Pfad
        // unterscheiden.
        let attendance_export_service = Arc::new(
            genossi_service_impl::attendance_export::AttendanceExportServiceImpl {
                transaction_dao: transaction_dao.clone(),
                permission_service: permission_service.clone(),
                assembly_dao: assembly_dao.clone(),
                attendance_dao: attendance_dao.clone(),
                pdf_generator: pdf_generator.clone(),
                template_base: Arc::new(template_storage.base_path().to_path_buf()),
            },
        );

        // Phase 11 (EXPO-01..03, EXPO-05): RepaymentExportServiceImpl (D-11 read-only,
        // D-10 status-gate, D-04 hardcoded purpose).
        // Re-uses the existing `pdf_generator` and `template_storage` Arcs
        // (same as Phase 6 attendance_export). KEINE neue PdfGenerator::new()-
        // Instanz und KEIN zweiter TemplateBase-Allokat. Alle DAOs werden via
        // Arc::clone aus den bereits konstruierten Arcs geteilt (Single-Arc-
        // per-Process pro Plan-10-Pattern).
        let repayment_export_service = Arc::new(
            genossi_service_impl::repayment_export::RepaymentExportServiceImpl::<
                RepaymentExportServiceDependencies,
            > {
                transaction_dao: transaction_dao.clone(),
                permission_service: permission_service.clone(),
                repayment_phase_dao: repayment_phase_dao.clone(),
                repayment_entry_dao: repayment_entry_dao.clone(),
                member_dao: member_dao.clone(),
                pdf_generator: pdf_generator.clone(),
                template_base: Arc::new(template_storage.base_path().to_path_buf()),
            },
        );

        // Phase 13 D-13-04 / D-13-10: RepaymentContextResolverImpl — shared
        // aggregation helper. KEIN neuer DAO-Arc: nutzt die existierenden
        // repayment_phase_dao + repayment_entry_dao Arcs (Single-Arc-per-
        // Process pro Plan-10 P07 Pattern).
        let repayment_context_resolver = Arc::new(
            genossi_service_impl::repayment_context::RepaymentContextResolverImpl::<
                RepaymentContextResolverDependencies,
            > {
                repayment_phase_dao: repayment_phase_dao.clone(),
                repayment_entry_dao: repayment_entry_dao.clone(),
            },
        );

        // Phase 13 D-13-01..11: RepaymentLetterServiceImpl. ALLE 10
        // Dependencies via .clone() von existierenden lokalen Arcs —
        // kein neuer DAO-Konstruktor (Single-Arc-per-Process, Plan 10 P07
        // Lektion). document_storage ist seit Phase 10 ein existierender
        // Single-Arc (lokales let auf line ~647).
        let repayment_letter_service = Arc::new(
            genossi_service_impl::repayment_letter::RepaymentLetterServiceImpl::<
                RepaymentLetterServiceDependencies,
            > {
                repayment_phase_dao: repayment_phase_dao.clone(),
                repayment_entry_dao: repayment_entry_dao.clone(),
                member_dao: member_dao.clone(),
                member_document_dao: member_document_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                transaction_dao: transaction_dao.clone(),
                uuid_service: uuid_service.clone(),
                repayment_context_resolver: repayment_context_resolver.clone(),
                document_storage: document_storage.clone(),
                pdf_generator: pdf_generator.clone(),
                template_base: Arc::new(template_storage.base_path().to_path_buf()),
            },
        );

        // Plan 02-07: HelperTokenServiceImpl with 8 deps (HelperTokenDao,
        // AssemblyDao, AuditLogDao, PermissionService, PermissionDao,
        // SessionService, UuidService, TransactionDao). assembly_dao is cloned
        // here from the same Arc that backs assembly_service above.
        let helper_token_service =
            Arc::new(genossi_service_impl::helper_token::HelperTokenServiceImpl {
                helper_token_dao,
                assembly_dao: assembly_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                permission_dao: permission_dao.clone(),
                session_service: session_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let mail_template_dao = Arc::new(MailTemplateDaoType::new(pool.clone()));
        // Phase 10 D-11: clone instead of move — the same Arc is persisted as
        // a RestStateImpl field below and shared with the mail-worker.
        let mail_template_service =
            Arc::new(MailTemplateServiceType::new(mail_template_dao.clone()));

        let static_document_dao_for_service = Arc::new(StaticDocumentDaoType::new(pool.clone()));
        let static_document_service = Arc::new(StaticDocumentServiceType::new(
            static_document_dao_for_service.clone(),
            document_storage.clone(),
        ));

        let backup_dao = Arc::new(BackupDao::new(pool.clone()));

        // Inbox service and worker wiring
        let inbox_dao = Arc::new(InboundMailDaoType::new(pool.clone()));
        let inbox_imap_client = Arc::new(InboxImapClientType::new());
        let inbox_config_dao = ConfigDao::new(pool.clone());
        let inbox_config_service = Arc::new(ConfigService::new(inbox_config_dao));
        let inbox_job_dao = Arc::new(MailJobDao::new(pool.clone()));
        let inbox_recipient_dao = Arc::new(MailRecipientDao::new(pool.clone()));
        // Phase 19: attachment DAO + storage are now part of InboxService.
        let inbox_attachment_dao = Arc::new(InboundMailAttachmentDaoType::new(pool.clone()));
        // Quick 260607-s0s: reply persists per-recipient and job-level
        // attachments — wire the same DAO types used by MailServiceImpl.
        // Pattern follows worker_attachment_dao / worker_static_attachment_dao
        // wiring further down (separate Arc per service, same pool).
        let inbox_recipient_attachment_dao =
            Arc::new(MailRecipientAttachmentDao::new(pool.clone()));
        let inbox_mail_job_static_attachment_dao =
            Arc::new(MailJobStaticAttachmentDaoType::new(pool.clone()));
        let inbox_service = Arc::new(genossi_mail::inbox::InboxServiceImpl::new(
            inbox_config_service.clone(),
            inbox_dao.clone(),
            inbox_imap_client.clone(),
            inbox_job_dao,
            inbox_recipient_dao,
            inbox_attachment_dao.clone(),
            document_storage.clone(),
            inbox_recipient_attachment_dao,
            inbox_mail_job_static_attachment_dao,
            static_document_dao_for_service.clone(),
        ));
        let worker_inbox_config_dao = ConfigDao::new(pool.clone());
        let worker_inbox_config_service = Arc::new(ConfigService::new(worker_inbox_config_dao));
        let worker_inbox_dao = Arc::new(InboundMailDaoType::new(pool.clone()));
        let worker_inbox_imap_client = Arc::new(InboxImapClientType::new());
        // Phase 19: worker also needs its own attachment DAO + storage handle.
        let worker_inbox_attachment_dao = Arc::new(InboundMailAttachmentDaoType::new(pool.clone()));
        let worker_inbox_storage = document_storage.clone();

        // Create separate instances for the worker (worker needs its own DAOs)
        let worker_job_dao = Arc::new(MailJobDao::new(pool.clone()));
        let worker_recipient_dao = Arc::new(MailRecipientDao::new(pool.clone()));
        let worker_attachment_dao = Arc::new(MailRecipientAttachmentDao::new(pool.clone()));
        let worker_static_attachment_dao =
            Arc::new(MailJobStaticAttachmentDaoType::new(pool.clone()));
        let worker_config_dao = ConfigDao::new(pool.clone());
        let worker_config_service = Arc::new(ConfigService::new(worker_config_dao));

        // Timestamp service
        let audit_timestamp_dao = AuditTimestampDao::new(pool.clone());
        let timestamp_config_dao = ConfigDao::new(pool.clone());
        let timestamp_config_service = Arc::new(ConfigService::new(timestamp_config_dao));
        let timestamp_service =
            Arc::new(genossi_service_impl::timestamp::TimestampServiceImpl::new(
                TransactionDao::new(pool.clone()),
                audit_timestamp_dao,
                AuditLogDao::new(pool.clone()),
                timestamp_config_service,
            ));

        // Backup worker dependencies
        let backup_config_dao = ConfigDao::new(pool.clone());
        let backup_config_service = Arc::new(ConfigService::new(backup_config_dao));
        let backup_sync_dao = Arc::new(BackupDocumentSyncDao::new(pool.clone()));
        let backup_comm_sync_dao = Arc::new(BackupCommunicationSyncDao::new(pool.clone()));

        Self {
            public_stats_cache: std::sync::Arc::new(
                genossi_rest::public_stats::PublicStatsCache::new(),
            ),
            member_service,
            member_import_service,
            member_action_service,
            membership_adjust_service,
            member_document_service,
            application_service,
            application_document_service,
            assembly_service,
            repayment_phase_service,
            repayment_entry_service,
            helper_token_service,
            attendance_service,
            attendance_export_service,
            // Phase 11 (EXPO-01..03, EXPO-05)
            repayment_export_service,
            // Phase 13 D-13-04 / D-13-10
            repayment_context_resolver,
            // Phase 13 D-13-01..11
            repayment_letter_service,
            audit_log_dao,
            // Phase 10 D-11: persist the worker-relevant DAOs (already
            // constructed above via Arc::new(XDao::new(pool.clone())) —
            // reuse those Arcs).
            member_document_dao,
            repayment_phase_dao,
            repayment_entry_dao,
            mail_template_dao,
            transaction_dao,
            timestamp_service,
            permission_service,
            session_service,
            document_storage,
            validation_service,
            user_preference_service,
            template_storage,
            pdf_generator,
            config_service,
            mail_service,
            mail_template_service,
            inbox_service,
            static_document_service,
            backup_dao,
            worker_inbox_config_service,
            worker_inbox_dao,
            worker_inbox_imap_client,
            worker_inbox_attachment_dao,
            worker_inbox_storage,
            worker_config_service,
            worker_job_dao,
            worker_recipient_dao,
            worker_attachment_dao,
            worker_static_attachment_dao,
            backup_config_service,
            backup_sync_dao,
            backup_comm_sync_dao,
            pool,
        }
    }
}

impl RestStateImpl {
    pub async fn initialize_audit_snapshot(&self) -> Result<(), Box<dyn std::error::Error>> {
        use genossi_dao::application::ApplicationDao;
        use genossi_dao::audit_log::AuditLogDao;

        use genossi_dao::member::MemberDao;
        use genossi_dao::member_action::MemberActionDao;
        use genossi_dao::member_document::MemberDocumentDao;
        use genossi_dao::{Transaction, TransactionDao};

        let transaction_dao = TransactionDaoImpl::new(self.pool.clone());
        let tx = transaction_dao
            .transaction()
            .await
            .map_err(|e| format!("Failed to start transaction: {:?}", e))?;

        // Check if audit_log is empty
        let latest = self
            .audit_log_dao
            .get_latest_hash(tx.clone())
            .await
            .map_err(|e| format!("Failed to check audit_log: {:?}", e))?;
        if latest.is_some() {
            tracing::info!("Audit log already has entries, skipping initial snapshot");
            return Ok(());
        }

        tracing::info!("Audit log is empty, creating initial snapshot of all entities...");

        let member_dao = genossi_dao_impl_sqlite::member::MemberDaoImpl::new(self.pool.clone());
        let member_action_dao =
            genossi_dao_impl_sqlite::member_action::MemberActionDaoImpl::new(self.pool.clone());
        let member_document_dao =
            genossi_dao_impl_sqlite::member_document::MemberDocumentDaoImpl::new(self.pool.clone());
        let application_dao =
            genossi_dao_impl_sqlite::application::ApplicationDaoImpl::new(self.pool.clone());

        let mut prev_hash = String::new();
        let mut total_entries = 0usize;

        // Snapshot all members
        let members = member_dao
            .dump_all(tx.clone())
            .await
            .map_err(|e| format!("Failed to load members: {:?}", e))?;
        let active_members: Vec<_> = members.iter().filter(|m| m.deleted.is_none()).collect();
        for member in &active_members {
            let entries = genossi_service_impl::audit_log::build_snapshot_entries(
                *member,
                "SYSTEM",
                "audit-snapshot",
                &prev_hash,
                &mut || uuid::Uuid::new_v4(),
            );
            if let Some(last) = entries.last() {
                prev_hash = last.entry_hash.to_string();
            }
            total_entries += entries.len();
            self.audit_log_dao
                .create_entries(&entries, tx.clone())
                .await
                .map_err(|e| format!("Failed to write member snapshot: {:?}", e))?;
        }
        tracing::info!("Snapshotted {} members", active_members.len());

        // Snapshot all member actions
        let actions = member_action_dao
            .dump_all(tx.clone())
            .await
            .map_err(|e| format!("Failed to load actions: {:?}", e))?;
        let active_actions: Vec<_> = actions.iter().filter(|a| a.deleted.is_none()).collect();
        for action in &active_actions {
            let entries = genossi_service_impl::audit_log::build_snapshot_entries(
                *action,
                "SYSTEM",
                "audit-snapshot",
                &prev_hash,
                &mut || uuid::Uuid::new_v4(),
            );
            if let Some(last) = entries.last() {
                prev_hash = last.entry_hash.to_string();
            }
            total_entries += entries.len();
            self.audit_log_dao
                .create_entries(&entries, tx.clone())
                .await
                .map_err(|e| format!("Failed to write action snapshot: {:?}", e))?;
        }
        tracing::info!("Snapshotted {} member actions", active_actions.len());

        // Snapshot all member documents
        let documents = member_document_dao
            .dump_all(tx.clone())
            .await
            .map_err(|e| format!("Failed to load documents: {:?}", e))?;
        let active_documents: Vec<_> = documents.iter().filter(|d| d.deleted.is_none()).collect();
        for document in &active_documents {
            let entries = genossi_service_impl::audit_log::build_snapshot_entries(
                *document,
                "SYSTEM",
                "audit-snapshot",
                &prev_hash,
                &mut || uuid::Uuid::new_v4(),
            );
            if let Some(last) = entries.last() {
                prev_hash = last.entry_hash.to_string();
            }
            total_entries += entries.len();
            self.audit_log_dao
                .create_entries(&entries, tx.clone())
                .await
                .map_err(|e| format!("Failed to write document snapshot: {:?}", e))?;
        }
        tracing::info!("Snapshotted {} member documents", active_documents.len());

        // Snapshot all applications
        let applications = application_dao
            .dump_all(tx.clone())
            .await
            .map_err(|e| format!("Failed to load applications: {:?}", e))?;
        let active_applications: Vec<_> = applications
            .iter()
            .filter(|a| a.deleted.is_none())
            .collect();
        for application in &active_applications {
            let entries = genossi_service_impl::audit_log::build_snapshot_entries(
                *application,
                "SYSTEM",
                "audit-snapshot",
                &prev_hash,
                &mut || uuid::Uuid::new_v4(),
            );
            if let Some(last) = entries.last() {
                prev_hash = last.entry_hash.to_string();
            }
            total_entries += entries.len();
            self.audit_log_dao
                .create_entries(&entries, tx.clone())
                .await
                .map_err(|e| format!("Failed to write application snapshot: {:?}", e))?;
        }
        tracing::info!("Snapshotted {} applications", active_applications.len());

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit snapshot transaction: {:?}", e))?;

        tracing::info!(
            "Initial audit snapshot complete: {} entries for {} members, {} actions, {} documents, {} applications",
            total_entries,
            active_members.len(),
            active_actions.len(),
            active_documents.len(),
            active_applications.len(),
        );

        Ok(())
    }

    pub fn start_inbox_worker(&self) {
        let config_service = self.worker_inbox_config_service.clone();
        let dao = self.worker_inbox_dao.clone();
        let imap_client = self.worker_inbox_imap_client.clone();
        let attachment_dao = self.worker_inbox_attachment_dao.clone();
        let storage = self.worker_inbox_storage.clone();
        tokio::spawn(async move {
            genossi_mail::inbox::start_inbox_worker(
                config_service,
                dao,
                imap_client,
                attachment_dao,
                storage,
            )
            .await;
        });
    }

    /// Phase 19 Plan 04: One-shot attachment backfill worker. Spawned at
    /// server start to retro-fit attachment rows for inbound mails received
    /// before Phase 19 introduced the attachment pipeline. Best-effort
    /// (D-05/D-06): mails whose IMAP UID can no longer be fetched are
    /// silently skipped. Idempotent on restart via the count_for_mail == 0
    /// filter inside `run_attachment_backfill`.
    pub fn start_attachment_backfill_worker(&self) {
        let config_service = self.worker_inbox_config_service.clone();
        let mail_dao = self.worker_inbox_dao.clone();
        let attachment_dao = self.worker_inbox_attachment_dao.clone();
        let storage = self.worker_inbox_storage.clone();
        let imap_client = self.worker_inbox_imap_client.clone();
        tokio::spawn(async move {
            genossi_mail::inbox::run_attachment_backfill(
                config_service,
                mail_dao,
                attachment_dao,
                storage,
                imap_client,
            )
            .await;
        });
    }

    pub fn start_backup_worker(&self) {
        let config_service = self.backup_config_service.clone();
        let backup_dao = self.backup_dao.clone();
        let sync_dao = self.backup_sync_dao.clone();
        let comm_sync_dao = self.backup_comm_sync_dao.clone();
        let document_storage = self.document_storage.clone();
        let audit_log_dao = Arc::new(AuditLogDao::new(self.pool.clone()));
        let audit_timestamp_dao = Arc::new(AuditTimestampDao::new(self.pool.clone()));
        let transaction_dao = Arc::new(TransactionDao::new(self.pool.clone()));
        tokio::spawn(async move {
            genossi_backup::worker::start_backup_worker(
                config_service,
                backup_dao,
                sync_dao,
                comm_sync_dao,
                document_storage,
                audit_log_dao,
                audit_timestamp_dao,
                transaction_dao,
            )
            .await;
        });
    }

    pub fn start_timestamp_worker(&self) {
        let timestamp_service = self.timestamp_service.clone();
        let config_dao = ConfigDao::new(self.pool.clone());
        let config_service = Arc::new(ConfigService::new(config_dao));
        tokio::spawn(async move {
            genossi_service_impl::timestamp_worker::start_timestamp_worker(
                timestamp_service,
                config_service,
            )
            .await;
        });
    }

    /// Phase 20 (Plan 02): Digest-Worker — pollt periodisch (~60s) und verschickt
    /// zur konfigurierten Uhrzeit eine Posteingangs-Digest-Mail pro Empfänger
    /// pro Kalendertag. Wiring spiegelt `start_timestamp_worker`; inbox_service
    /// und mail_service sind bereits vorhandene Felder, der DigestStateDao wird
    /// ad-hoc aus dem Pool gebaut (Pattern start_mail_worker).
    pub fn start_digest_worker(&self) {
        let config_dao = ConfigDao::new(self.pool.clone());
        let config_service = Arc::new(ConfigService::new(config_dao));
        let inbox_service = self.inbox_service.clone();
        let mail_service = self.mail_service.clone();
        let digest_state_dao = Arc::new(DigestStateDaoType::new(self.pool.clone()));
        tokio::spawn(async move {
            genossi_mail::digest::start_digest_worker(
                config_service,
                inbox_service,
                mail_service,
                digest_state_dao,
            )
            .await;
        });
    }

    pub fn start_mail_worker(&self) {
        let config_service = self.worker_config_service.clone();
        let job_dao = self.worker_job_dao.clone();
        let recipient_dao = self.worker_recipient_dao.clone();
        let attachment_dao = self.worker_attachment_dao.clone();
        let static_attachment_dao = self.worker_static_attachment_dao.clone();
        let document_storage = self.document_storage.clone();
        let member_resolver = Arc::new(PoolMemberResolver::new(self.pool.clone()));
        let inbound_mail_dao = Arc::new(InboundMailDaoType::new(self.pool.clone()));
        // Phase 10 D-11: 6 new deps for repayment-context aggregation +
        // auditable MemberDocument-create. All DAOs are shared via
        // Arc::clone from RestStateImpl fields (added in Task 1). The
        // audit_log_dao is the SAME Arc that all other audited services
        // use — this guarantees the worker contributes to the single
        // per-process hash chain (T-10-07-02 mitigation).
        let member_document_dao = self.member_document_dao.clone();
        let audit_log_dao = self.audit_log_dao.clone();
        let mail_template_dao = self.mail_template_dao.clone();
        let repayment_entry_dao = self.repayment_entry_dao.clone();
        let repayment_phase_dao = self.repayment_phase_dao.clone();
        let transaction_dao = self.transaction_dao.clone();
        // Quick 260603-h0r: Shared aggregation resolver — same Arc as Letter-Service.
        let repayment_context_resolver = self.repayment_context_resolver.clone();
        tokio::spawn(async move {
            genossi_mail::worker::start_mail_worker(
                config_service,
                job_dao,
                recipient_dao,
                attachment_dao,
                static_attachment_dao,
                document_storage,
                member_resolver,
                inbound_mail_dao,
                // Phase 10 D-11 new args (same positional order as
                // worker.rs signature: MD, AL, MT, RE, RP, TX).
                member_document_dao,
                audit_log_dao,
                mail_template_dao,
                repayment_entry_dao,
                repayment_phase_dao,
                transaction_dao,
                // Quick 260603-h0r: 15th positional arg (RCR generic).
                repayment_context_resolver,
            )
            .await;
        });
    }

    /// Quick 260614-b1t: one-shot startup backfill that retroactively renders
    /// legacy mail_recipients rows (rendered_subject/body still NULL) and marks
    /// them rendered_reconstructed=true. Shares the exact same render function as
    /// start_mail_worker (DRY). Idempotent: a second run after a full fill is a
    /// no-op because find_recipients_without_rendered only returns NULL rows.
    pub fn start_rendered_backfill_worker(&self) {
        let recipient_dao = self.worker_recipient_dao.clone();
        let job_dao = self.worker_job_dao.clone();
        let member_resolver = Arc::new(PoolMemberResolver::new(self.pool.clone()));
        let repayment_entry_dao = self.repayment_entry_dao.clone();
        let repayment_phase_dao = self.repayment_phase_dao.clone();
        let transaction_dao = self.transaction_dao.clone();
        let repayment_context_resolver = self.repayment_context_resolver.clone();
        tokio::spawn(async move {
            genossi_mail::backfill::run_rendered_backfill(
                recipient_dao,
                job_dao,
                member_resolver,
                repayment_entry_dao,
                repayment_phase_dao,
                transaction_dao,
                repayment_context_resolver,
            )
            .await;
        });
    }
}

impl genossi_mail::inbox_rest::InboxRestState for RestStateImpl {
    type InboxService = InboxServiceType;
    fn inbox_service(&self) -> Arc<Self::InboxService> {
        self.inbox_service.clone()
    }
    fn resolve_member_name(
        &self,
        member_id: UuidType,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            use genossi_dao::member::MemberDao as _;
            use genossi_dao::TransactionDao as _;
            let transaction_dao = TransactionDaoImpl::new(pool.clone());
            let member_dao = genossi_dao_impl_sqlite::member::MemberDaoImpl::new(pool);
            let tx = transaction_dao.transaction().await.ok()?;
            let m = member_dao.find_by_id(member_id, tx).await.ok()??;
            Some(format!("{} {}", m.first_name, m.last_name))
        })
    }
    // Phase 19 D-08: expose the existing FilesystemDocumentStorage to the
    // attachment download handler in `genossi_mail`. Single Arc shared
    // with member_document/static_document services (same per-process
    // instance — D-04 path-traversal protection lives in the storage impl).
    // Named `inbox_document_storage` (not `document_storage`) to avoid
    // a method-resolution clash with `RestStateDef::document_storage`.
    fn inbox_document_storage(
        &self,
    ) -> Arc<dyn genossi_service::document_storage::DocumentStorage> {
        self.document_storage.clone()
    }
    // Phase 19 (T-02): delegate Content-Disposition header building to the
    // canonical helpers in `genossi_rest::http_util`. `genossi_mail` can
    // not import `genossi_rest` directly (would create a circular crate
    // dep), so we trampoline through the trait.
    fn content_disposition_attachment(&self, filename: &str) -> String {
        genossi_rest::http_util::content_disposition_attachment(filename)
    }
    fn content_disposition_inline(&self, filename: &str) -> String {
        genossi_rest::http_util::content_disposition_inline(filename)
    }
    // Quick 260607-s0s: delegate to the same SQL query as the MailRestState
    // impl (resolve_document below) — the reply handler needs it for the
    // attachment-ownership check. Inline-duplication keeps the diff minimal;
    // the helper lives entirely on RestStateImpl.
    fn resolve_document(
        &self,
        document_id: UuidType,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Option<genossi_mail::rest::ResolvedDocument>>
                + Send
                + '_,
        >,
    > {
        let pool = self.pool.clone();
        Box::pin(async move {
            let id_bytes = document_id.as_bytes().to_vec();
            let row: Option<(Vec<u8>, String, String, String)> = sqlx::query_as(
                "SELECT member_id, file_name, mime_type, relative_path \
                 FROM member_document WHERE id = ? AND deleted IS NULL",
            )
            .bind(id_bytes)
            .fetch_optional(pool.as_ref())
            .await
            .ok()?;

            let (member_id_bytes, file_name, mime_type, relative_path) = row?;
            let member_id = UuidType::from_slice(&member_id_bytes).ok()?;

            Some(genossi_mail::rest::ResolvedDocument {
                document_id,
                member_id,
                file_name,
                mime_type,
                relative_path,
            })
        })
    }
}

impl genossi_mail::rest_templates::MailTemplateRestState for RestStateImpl {
    type MailTemplateService = MailTemplateServiceType;
    fn mail_template_service(&self) -> Arc<Self::MailTemplateService> {
        self.mail_template_service.clone()
    }
}

impl genossi_mail::rest::MailRestState for RestStateImpl {
    type MailService = MailServiceType;
    fn mail_service(&self) -> Arc<Self::MailService> {
        self.mail_service.clone()
    }
    fn resolve_member(
        &self,
        member_id: UuidType,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Option<genossi_dao::member::MemberEntity>> + Send + '_,
        >,
    > {
        let pool = self.pool.clone();
        Box::pin(async move {
            use genossi_dao::member::MemberDao as _;
            use genossi_dao::TransactionDao as _;
            let transaction_dao = TransactionDaoImpl::new(pool.clone());
            let member_dao = genossi_dao_impl_sqlite::member::MemberDaoImpl::new(pool);
            let tx = transaction_dao.transaction().await.ok()?;
            member_dao.find_by_id(member_id, tx).await.ok()?
        })
    }
    fn resolve_members(
        &self,
        member_ids: &[UuidType],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Vec<genossi_dao::member::MemberEntity>> + Send + '_>,
    > {
        let pool = self.pool.clone();
        let ids = member_ids.to_vec();
        Box::pin(async move {
            use genossi_dao::member::MemberDao as _;
            use genossi_dao::TransactionDao as _;
            let transaction_dao = TransactionDaoImpl::new(pool.clone());
            let member_dao = genossi_dao_impl_sqlite::member::MemberDaoImpl::new(pool);
            let tx = match transaction_dao.transaction().await {
                Ok(tx) => tx,
                Err(_) => return vec![],
            };
            let all = match member_dao.all(tx).await {
                Ok(all) => all,
                Err(_) => return vec![],
            };
            all.iter()
                .filter(|m| ids.contains(&m.id))
                .cloned()
                .collect()
        })
    }
    /// Quick-c19: mirror of `genossi_mail/src/worker.rs:332-361` for the
    /// `/api/mail/preview` endpoint. Aggregates the member's Open/Contacted
    /// RepaymentEntries in the given phase and formats payout_amount in the
    /// German locale (`X,YZ`). Returns `None` if the phase doesn't exist OR
    /// the member has no relevant entries (D-05 symmetry with the worker —
    /// caller must use `{% if share_count is defined %}` guards).
    fn resolve_repayment_context(
        &self,
        phase_id: UuidType,
        member_id: UuidType,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Option<(String, i32, String, i32)>> + Send + '_>,
    > {
        let pool = self.pool.clone();
        Box::pin(async move {
            use genossi_dao::repayment_entry::{RepaymentEntryDao as _, RepaymentEntryStatus};
            use genossi_dao::repayment_phase::RepaymentPhaseDao as _;
            use genossi_dao::TransactionDao as _;

            let transaction_dao = TransactionDaoImpl::new(pool.clone());
            let phase_dao =
                genossi_dao_impl_sqlite::repayment_phase::RepaymentPhaseDaoImpl::new(pool.clone());
            let entry_dao =
                genossi_dao_impl_sqlite::repayment_entry::RepaymentEntryDaoImpl::new(pool);

            let tx = transaction_dao.transaction().await.ok()?;
            // TransactionImpl is Clone (genossi_dao_impl_sqlite/src/transaction.rs:7),
            // verified for this implementation. Reuse the same tx for both
            // read-only DAO calls — identical pattern to the worker
            // (worker.rs aggregation block reuses agg_tx for phase + entries).
            let phase = phase_dao.find_by_id(phase_id, tx.clone()).await.ok()??;
            let entries = entry_dao.find_by_phase_id(phase_id, tx).await.ok()?;

            // Mirror worker.rs:332-361 EXACTLY — D-06 filter + D-05 emptiness check.
            let share_count: i32 = entries
                .iter()
                .filter(|e| {
                    e.deleted.is_none()
                        && e.member_id == member_id
                        && matches!(
                            e.status,
                            RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
                        )
                })
                .map(|e| e.share_count_to_pay_out)
                .sum();

            if share_count == 0 {
                return None;
            }

            let cents: i64 = (share_count as i64) * phase.share_value;
            // German locale "X,YZ" — identical format string as worker.rs:353.
            let payout_amount = format!("{},{:02}", cents / 100, cents % 100);
            // Quick 260602-r2i: share_value (phase-wide Anteilswert) als Euro-String.
            let share_value_str =
                format!("{},{:02}", phase.share_value / 100, phase.share_value % 100);
            Some((
                payout_amount,
                share_count,
                share_value_str,
                phase.fiscal_year,
            ))
        })
    }
    fn resolve_document(
        &self,
        document_id: UuidType,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Option<genossi_mail::rest::ResolvedDocument>>
                + Send
                + '_,
        >,
    > {
        let pool = self.pool.clone();
        Box::pin(async move {
            let id_bytes = document_id.as_bytes().to_vec();
            let row: Option<(Vec<u8>, String, String, String)> = sqlx::query_as(
                "SELECT member_id, file_name, mime_type, relative_path \
                 FROM member_document WHERE id = ? AND deleted IS NULL",
            )
            .bind(id_bytes)
            .fetch_optional(pool.as_ref())
            .await
            .ok()?;

            let (member_id_bytes, file_name, mime_type, relative_path) = row?;
            let member_id = UuidType::from_slice(&member_id_bytes).ok()?;

            Some(genossi_mail::rest::ResolvedDocument {
                document_id,
                member_id,
                file_name,
                mime_type,
                relative_path,
            })
        })
    }
    fn get_recipient_attachments(
        &self,
        recipient_id: UuidType,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Vec<genossi_mail::rest::MailAttachmentTO>> + Send + '_,
        >,
    > {
        let attachment_dao = self.worker_attachment_dao.clone();
        Box::pin(async move {
            use genossi_mail::dao::MailRecipientAttachmentDao;
            match attachment_dao.find_by_recipient_id(recipient_id).await {
                Ok(atts) => atts
                    .iter()
                    .map(|a| genossi_mail::rest::MailAttachmentTO {
                        document_id: a.document_id.to_string(),
                        file_name: a.file_name.to_string(),
                    })
                    .collect(),
                Err(_) => vec![],
            }
        })
    }
}

impl genossi_mail::communication_rest::CommunicationRestState for RestStateImpl {
    type CommunicationDao = genossi_mail::dao_sqlite::CommunicationDaoSqlite;
    fn communication_dao(&self) -> Arc<Self::CommunicationDao> {
        Arc::new(genossi_mail::dao_sqlite::CommunicationDaoSqlite::new(
            self.pool.clone(),
        ))
    }
}

impl genossi_rest::public_stats::PublicStatsState for RestStateImpl {
    fn public_stats_cache(&self) -> &genossi_rest::public_stats::PublicStatsCache {
        &self.public_stats_cache
    }

    fn get_public_stats_enabled(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<bool>> + Send + '_>> {
        let config_service = self.config_service.clone();
        Box::pin(async move {
            use genossi_config::service::ConfigService;
            match config_service.get("public_stats_enabled").await {
                Ok(entry) => Some(entry.value.as_ref() == "true"),
                Err(_) => Some(false),
            }
        })
    }

    fn get_active_member_count(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<u64>> + Send + '_>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            use genossi_dao::member::MemberDao as _;
            use genossi_dao::TransactionDao as _;
            let transaction_dao = TransactionDaoImpl::new(pool.clone());
            let member_dao = genossi_dao_impl_sqlite::member::MemberDaoImpl::new(pool);
            let tx = transaction_dao.transaction().await.ok()?;
            let today = time::OffsetDateTime::now_utc().date();
            member_dao.count_active(today, tx).await.ok()
        })
    }
}

impl genossi_rest::application::ApplicationRestState for RestStateImpl {
    type ApplicationService = ApplicationService;

    fn application_service(&self) -> Arc<Self::ApplicationService> {
        self.application_service.clone()
    }

    fn get_config_value(
        &self,
        key: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>> {
        let config_service = self.config_service.clone();
        let key = key.to_string();
        Box::pin(async move {
            use genossi_config::service::ConfigService;
            match config_service.get(&key).await {
                Ok(entry) => Some(entry.value.to_string()),
                Err(_) => None,
            }
        })
    }
}

impl genossi_rest::assembly::AssemblyRestState for RestStateImpl {
    type AssemblyService = AssemblyService;

    fn assembly_service(&self) -> Arc<Self::AssemblyService> {
        self.assembly_service.clone()
    }
}

// Phase 7 Plan 04: bind RepaymentPhaseServiceImpl to the REST handlers
// generated in genossi_rest::repayment_phase.
impl genossi_rest::repayment_phase::RepaymentPhaseRestState for RestStateImpl {
    type RepaymentPhaseService = RepaymentPhaseService;

    fn repayment_phase_service(&self) -> Arc<Self::RepaymentPhaseService> {
        self.repayment_phase_service.clone()
    }
}

// Phase 8 Plan 05: bind RepaymentEntryServiceImpl to the REST handlers
// generated in genossi_rest::repayment_entry.
impl genossi_rest::repayment_entry::RepaymentEntryRestState for RestStateImpl {
    type RepaymentEntryService = RepaymentEntryService;

    fn repayment_entry_service(&self) -> Arc<Self::RepaymentEntryService> {
        self.repayment_entry_service.clone()
    }
}

impl genossi_rest::helper_token::HelperTokenRestState for RestStateImpl {
    type HelperTokenService = HelperTokenService;

    fn helper_token_service(&self) -> Arc<Self::HelperTokenService> {
        self.helper_token_service.clone()
    }
}

impl genossi_rest::attendance::AttendanceRestState for RestStateImpl {
    type AttendanceService = AttendanceService;

    fn attendance_service(&self) -> Arc<Self::AttendanceService> {
        self.attendance_service.clone()
    }
}

// Phase 6 Plan 03: bind AttendanceExportServiceImpl to the REST handlers
// generated in genossi_rest::attendance_export.
impl genossi_rest::attendance_export::AttendanceExportRestState for RestStateImpl {
    type AttendanceExportService = AttendanceExportService;

    fn attendance_export_service(&self) -> Arc<Self::AttendanceExportService> {
        self.attendance_export_service.clone()
    }
}

// Phase 11 (EXPO-01..03, EXPO-05): expose RepaymentExportService to REST handlers
// generated in genossi_rest::repayment_export.
impl genossi_rest::repayment_export::RepaymentExportRestState for RestStateImpl {
    type RepaymentExportService = RepaymentExportService;

    fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService> {
        self.repayment_export_service.clone()
    }
}

// Phase 13 D-13-01..11: expose RepaymentLetterService to REST handlers
// generated in genossi_rest::repayment_letter (POST /letters/generate).
impl genossi_rest::repayment_letter::RepaymentLetterRestState for RestStateImpl {
    type RepaymentLetterService = RepaymentLetterService;

    fn repayment_letter_service(&self) -> Arc<Self::RepaymentLetterService> {
        self.repayment_letter_service.clone()
    }
}

impl genossi_config::rest::ConfigRestState for RestStateImpl {
    type ConfigService = ConfigService;
    fn config_service(&self) -> Arc<Self::ConfigService> {
        self.config_service.clone()
    }
}

impl genossi_rest::audit_timestamp::TimestampRestState for RestStateImpl {
    type TimestampService = TimestampServiceType;

    fn timestamp_service(&self) -> Arc<Self::TimestampService> {
        self.timestamp_service.clone()
    }
}

impl genossi_rest::audit_log::AuditRestState for RestStateImpl {
    type AuditLogDao = AuditLogDao;

    fn audit_log_dao(&self) -> Arc<Self::AuditLogDao> {
        self.audit_log_dao.clone()
    }

    fn audit_transaction(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Transaction, genossi_dao::DaoError>>
                + Send
                + '_,
        >,
    > {
        let pool = self.pool.clone();
        Box::pin(async move { genossi_dao_impl_sqlite::TransactionImpl::new(&pool).await })
    }
}

impl genossi_rest::RestStateDef for RestStateImpl {
    type MemberService = MemberService;
    type PermissionService = PermissionService;
    type SessionService = SessionService;
    type MemberImportService = MemberImportService;
    type MemberActionService = MemberActionService;
    type MembershipAdjustService = MembershipAdjustService;
    type MemberDocumentService = MemberDocumentService;
    // Phase 25 Wave 3 (Plan 25-04).
    type ApplicationDocumentService = ApplicationDocumentService;
    type DocumentStorage = DocumentStorage;
    type ValidationService = ValidationService;
    type UserPreferenceService = UserPreferenceService;
    type StaticDocumentService = StaticDocumentServiceType;
    type BackupDao = BackupDao;

    fn member_service(&self) -> Arc<Self::MemberService> {
        self.member_service.clone()
    }

    fn permission_service(&self) -> Arc<Self::PermissionService> {
        self.permission_service.clone()
    }

    fn session_service(&self) -> Arc<Self::SessionService> {
        self.session_service.clone()
    }

    fn member_import_service(&self) -> Arc<Self::MemberImportService> {
        self.member_import_service.clone()
    }

    fn member_action_service(&self) -> Arc<Self::MemberActionService> {
        self.member_action_service.clone()
    }

    fn membership_adjust_service(&self) -> Arc<Self::MembershipAdjustService> {
        self.membership_adjust_service.clone()
    }

    fn member_document_service(&self) -> Arc<Self::MemberDocumentService> {
        self.member_document_service.clone()
    }

    fn application_document_service(&self) -> Arc<Self::ApplicationDocumentService> {
        self.application_document_service.clone()
    }

    fn document_storage(&self) -> Arc<Self::DocumentStorage> {
        self.document_storage.clone()
    }

    fn validation_service(&self) -> Arc<Self::ValidationService> {
        self.validation_service.clone()
    }

    fn user_preference_service(&self) -> Arc<Self::UserPreferenceService> {
        self.user_preference_service.clone()
    }

    fn static_document_service(&self) -> Arc<Self::StaticDocumentService> {
        self.static_document_service.clone()
    }

    fn template_storage(&self) -> Arc<genossi_service_impl::template_storage::TemplateStorage> {
        self.template_storage.clone()
    }

    fn pdf_generator(&self) -> Arc<genossi_service_impl::pdf_generation::PdfGenerator> {
        self.pdf_generator.clone()
    }

    fn backup_dao(&self) -> Arc<Self::BackupDao> {
        self.backup_dao.clone()
    }
}
