use std::sync::Arc;

use genossi_dao_impl_sqlite::{TransactionDaoImpl, TransactionImpl};
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

        Self {
            member_service,
            member_import_service,
            member_action_service,
            member_document_service,
            permission_service,
            session_service,
            document_storage,
            validation_service,
        }
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
}
