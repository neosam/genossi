use std::sync::Arc;

use genossi_dao_impl_sqlite::{TransactionDaoImpl, TransactionImpl};
use uuid::Uuid as UuidType;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::permission::MockContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::user_service::MockUserService;
#[cfg(feature = "oidc")]
use genossi_service::auth_types::AuthenticatedContext;
use genossi_service_impl::member::MemberServiceDeps;
use genossi_service_impl::member_action::MemberActionServiceDeps;
use genossi_service_impl::member_document::MemberDocumentServiceDeps;
use genossi_service_impl::member_import::MemberImportServiceDeps;
use genossi_service_impl::permission::PermissionServiceDeps;
use genossi_service_impl::user_preference::UserPreferenceServiceDeps;
use genossi_service_impl::validation::ValidationServiceDeps;
use sqlx::SqlitePool;

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
type SessionService =
    genossi_service_impl::session::SessionServiceImpl<SessionServiceDependencies>;

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
}

pub struct MemberServiceDependencies;

unsafe impl Send for MemberServiceDependencies {}
unsafe impl Sync for MemberServiceDependencies {}

impl MemberServiceDeps for MemberServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberService =
    genossi_service_impl::member::MemberServiceImpl<MemberServiceDependencies>;

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

type MemberActionDao = genossi_dao_impl_sqlite::member_action::MemberActionDaoImpl;
type MemberDocumentDao = genossi_dao_impl_sqlite::member_document::MemberDocumentDaoImpl;

pub struct MemberActionServiceDependencies;

unsafe impl Send for MemberActionServiceDependencies {}
unsafe impl Sync for MemberActionServiceDependencies {}

impl MemberActionServiceDeps for MemberActionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberActionDao = MemberActionDao;
    type MemberDao = MemberDao;
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
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberDocumentService =
    genossi_service_impl::member_document::MemberDocumentServiceImpl<MemberDocumentServiceDependencies>;

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

type UserPreferenceService =
    genossi_service_impl::user_preference::UserPreferenceServiceImpl<UserPreferenceServiceDependencies>;

type ConfigDao = genossi_config::dao_sqlite::ConfigDaoSqlite;
type ConfigService = genossi_config::service::ConfigServiceImpl<ConfigDao>;
type MailJobDao = genossi_mail::dao_sqlite::MailJobDaoSqlite;
type MailRecipientDao = genossi_mail::dao_sqlite::MailRecipientDaoSqlite;
type MailRecipientAttachmentDao = genossi_mail::dao_sqlite::MailRecipientAttachmentDaoSqlite;
type MailServiceType = genossi_mail::service::MailServiceImpl<ConfigService, MailJobDao, MailRecipientDao, MailRecipientAttachmentDao>;

// RestStateImpl with all services
#[derive(Clone)]
pub struct RestStateImpl {
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
    // Worker dependencies (kept for spawning the background worker)
    worker_config_service: Arc<ConfigService>,
    worker_job_dao: Arc<MailJobDao>,
    worker_recipient_dao: Arc<MailRecipientDao>,
    worker_attachment_dao: Arc<MailRecipientAttachmentDao>,
    // Pool for direct document resolution queries
    pool: Arc<SqlitePool>,
}

impl RestStateImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        // Create DAOs
        let transaction_dao = Arc::new(TransactionDao::new(pool.clone()));
        let member_dao = Arc::new(MemberDao::new(pool.clone()));
        let permission_dao =
            Arc::new(genossi_dao_impl_sqlite::permission::PermissionDaoImpl::new(pool.clone()));

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

        let member_service =
            Arc::new(genossi_service_impl::member::MemberServiceImpl {
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let member_action_service =
            Arc::new(genossi_service_impl::member_action::MemberActionServiceImpl {
                member_action_dao: member_action_dao.clone(),
                member_dao: member_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let member_document_dao = Arc::new(MemberDocumentDao::new(pool.clone()));

        let member_document_service =
            Arc::new(genossi_service_impl::member_document::MemberDocumentServiceImpl {
                member_document_dao,
                member_dao: member_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let document_storage = Arc::new(DocumentStorage::from_env());

        let validation_service =
            Arc::new(genossi_service_impl::validation::ValidationServiceImpl {
                member_dao: member_dao.clone(),
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let user_preference_dao = Arc::new(UserPreferenceDao::new(pool.clone()));

        let user_preference_service =
            Arc::new(genossi_service_impl::user_preference::UserPreferenceServiceImpl {
                user_preference_dao,
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            });

        let member_import_service =
            Arc::new(genossi_service_impl::member_import::MemberImportServiceImpl {
                member_dao,
                member_action_dao: member_action_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service,
                transaction_dao,
            });

        #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
        let session_service = Arc::new(genossi_service_impl::session::MockSessionServiceImpl);

        #[cfg(feature = "oidc")]
        let session_service = Arc::new(genossi_service_impl::session::SessionServiceImpl {
            permission_dao: permission_dao.clone(),
        });

        let template_storage =
            Arc::new(genossi_service_impl::template_storage::TemplateStorage::from_env());
        let pdf_generator =
            Arc::new(genossi_service_impl::pdf_generation::PdfGenerator::new());

        let config_dao = ConfigDao::new(pool.clone());
        let config_service = Arc::new(ConfigService::new(config_dao));

        let mail_job_dao = MailJobDao::new(pool.clone());
        let mail_recipient_dao = MailRecipientDao::new(pool.clone());
        let mail_attachment_dao = MailRecipientAttachmentDao::new(pool.clone());
        let config_dao_for_mail = ConfigDao::new(pool.clone());
        let config_service_for_mail = ConfigService::new(config_dao_for_mail);
        let mail_service = Arc::new(MailServiceType::new(
            config_service_for_mail,
            mail_job_dao,
            mail_recipient_dao,
            mail_attachment_dao,
        ));

        // Create separate instances for the worker (worker needs its own DAOs)
        let worker_job_dao = Arc::new(MailJobDao::new(pool.clone()));
        let worker_recipient_dao = Arc::new(MailRecipientDao::new(pool.clone()));
        let worker_attachment_dao = Arc::new(MailRecipientAttachmentDao::new(pool.clone()));
        let worker_config_dao = ConfigDao::new(pool.clone());
        let worker_config_service = Arc::new(ConfigService::new(worker_config_dao));

        Self {
            member_service,
            member_import_service,
            member_action_service,
            member_document_service,
            permission_service,
            session_service,
            document_storage,
            validation_service,
            user_preference_service,
            template_storage,
            pdf_generator,
            config_service,
            mail_service,
            worker_config_service,
            worker_job_dao,
            worker_recipient_dao,
            worker_attachment_dao,
            pool,
        }
    }
}

impl RestStateImpl {
    pub fn start_mail_worker(&self) {
        let config_service = self.worker_config_service.clone();
        let job_dao = self.worker_job_dao.clone();
        let recipient_dao = self.worker_recipient_dao.clone();
        let attachment_dao = self.worker_attachment_dao.clone();
        let document_storage = self.document_storage.clone();
        tokio::spawn(async move {
            genossi_mail::worker::start_mail_worker(
                config_service,
                job_dao,
                recipient_dao,
                attachment_dao,
                document_storage,
            )
            .await;
        });
    }
}

impl genossi_mail::rest::MailRestState for RestStateImpl {
    type MailService = MailServiceType;
    fn mail_service(&self) -> Arc<Self::MailService> {
        self.mail_service.clone()
    }
    fn resolve_document(
        &self,
        document_id: UuidType,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Option<genossi_mail::rest::ResolvedDocument>,
                > + Send
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
            dyn std::future::Future<Output = Vec<genossi_mail::rest::MailAttachmentTO>>
                + Send
                + '_,
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

impl genossi_config::rest::ConfigRestState for RestStateImpl {
    type ConfigService = ConfigService;
    fn config_service(&self) -> Arc<Self::ConfigService> {
        self.config_service.clone()
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

    fn template_storage(&self) -> Arc<genossi_service_impl::template_storage::TemplateStorage> {
        self.template_storage.clone()
    }

    fn pdf_generator(&self) -> Arc<genossi_service_impl::pdf_generation::PdfGenerator> {
        self.pdf_generator.clone()
    }
}
