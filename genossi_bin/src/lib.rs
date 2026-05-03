use std::sync::Arc;

use genossi_dao_impl_sqlite::{TransactionDaoImpl, TransactionImpl};
#[cfg(feature = "oidc")]
use genossi_service::auth_types::AuthenticatedContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::permission::MockContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::user_service::MockUserService;
use genossi_service_impl::application::ApplicationServiceDeps;
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
}

type AssemblyService =
    genossi_service_impl::assembly::AssemblyServiceImpl<AssemblyServiceDependencies>;

type HelperTokenDao = genossi_dao_impl_sqlite::helper_token::HelperTokenDaoImpl;

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
type InboxImapClientType = genossi_mail::inbox_imap::AsyncImapClient;
type InboxServiceType = genossi_mail::inbox::InboxServiceImpl<
    ConfigService,
    InboundMailDaoType,
    InboxImapClientType,
    MailJobDao,
    MailRecipientDao,
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
    assembly_service: Arc<AssemblyService>,
    helper_token_service: Arc<HelperTokenService>,
    audit_log_dao: Arc<AuditLogDao>,
    timestamp_service: Arc<TimestampServiceType>,
    backup_dao: Arc<BackupDao>,
    // Inbox worker dependencies
    worker_inbox_config_service: Arc<ConfigService>,
    worker_inbox_dao: Arc<InboundMailDaoType>,
    worker_inbox_imap_client: Arc<InboxImapClientType>,
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

        let member_document_dao = Arc::new(MemberDocumentDao::new(pool.clone()));

        let member_document_service = Arc::new(
            genossi_service_impl::member_document::MemberDocumentServiceImpl {
                member_document_dao,
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
            Arc::new(genossi_service_impl::session::MockSessionServiceImpl::with_probe(probe))
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
                application_dao,
                audit_log_dao: audit_log_dao.clone(),
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
                config_service: config_service_for_app,
                mail_service: mail_service.clone(),
            });

        // assembly_dao was already constructed above (before session_service).
        let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
        let assembly_service = Arc::new(genossi_service_impl::assembly::AssemblyServiceImpl {
            assembly_dao: assembly_dao.clone(),
            assembly_member_snapshot_dao,
            member_dao: member_dao.clone(),
            audit_log_dao: audit_log_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Plan 02-07: HelperTokenServiceImpl with 8 deps (HelperTokenDao,
        // AssemblyDao, AuditLogDao, PermissionService, PermissionDao,
        // SessionService, UuidService, TransactionDao). assembly_dao is cloned
        // here from the same Arc that backs assembly_service above.
        let helper_token_dao = Arc::new(HelperTokenDao::new(pool.clone()));
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
        let mail_template_service = Arc::new(MailTemplateServiceType::new(mail_template_dao));

        let static_document_dao_for_service = Arc::new(StaticDocumentDaoType::new(pool.clone()));
        let static_document_service = Arc::new(StaticDocumentServiceType::new(
            static_document_dao_for_service,
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
        let inbox_service = Arc::new(genossi_mail::inbox::InboxServiceImpl::new(
            inbox_config_service.clone(),
            inbox_dao.clone(),
            inbox_imap_client.clone(),
            inbox_job_dao,
            inbox_recipient_dao,
        ));
        let worker_inbox_config_dao = ConfigDao::new(pool.clone());
        let worker_inbox_config_service = Arc::new(ConfigService::new(worker_inbox_config_dao));
        let worker_inbox_dao = Arc::new(InboundMailDaoType::new(pool.clone()));
        let worker_inbox_imap_client = Arc::new(InboxImapClientType::new());

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
            member_document_service,
            application_service,
            assembly_service,
            helper_token_service,
            audit_log_dao,
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
        use genossi_dao::auditable::Auditable;
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
        tokio::spawn(async move {
            genossi_mail::inbox::start_inbox_worker(config_service, dao, imap_client).await;
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

    pub fn start_mail_worker(&self) {
        let config_service = self.worker_config_service.clone();
        let job_dao = self.worker_job_dao.clone();
        let recipient_dao = self.worker_recipient_dao.clone();
        let attachment_dao = self.worker_attachment_dao.clone();
        let static_attachment_dao = self.worker_static_attachment_dao.clone();
        let document_storage = self.document_storage.clone();
        let member_resolver = Arc::new(PoolMemberResolver::new(self.pool.clone()));
        let inbound_mail_dao = Arc::new(InboundMailDaoType::new(self.pool.clone()));
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

impl genossi_rest::helper_token::HelperTokenRestState for RestStateImpl {
    type HelperTokenService = HelperTokenService;

    fn helper_token_service(&self) -> Arc<Self::HelperTokenService> {
        self.helper_token_service.clone()
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
    type MemberDocumentService = MemberDocumentService;
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

    fn member_document_service(&self) -> Arc<Self::MemberDocumentService> {
        self.member_document_service.clone()
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
