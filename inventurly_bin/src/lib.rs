use std::sync::Arc;

use inventurly_dao_impl_sqlite::{person::PersonDaoImpl, product::ProductDaoImpl, TransactionDaoImpl, TransactionImpl};
use inventurly_service::permission::MockContext;
use inventurly_service::user_service::MockUserService;
use inventurly_service_impl::{
    permission::PermissionServiceDeps,
    person::{PersonServiceDeps, PersonServiceImpl},
    product::{ProductServiceDeps, ProductServiceImpl},
    csv_import::{CsvImportServiceDeps, CsvImportServiceImpl},
    duplicate_detection::{DuplicateDetectionServiceDeps, DuplicateDetectionServiceImpl},
};
use sqlx::SqlitePool;

// Type aliases for clarity
type Context = MockContext;
type Transaction = TransactionImpl;
type TransactionDao = TransactionDaoImpl;
type PersonDao = PersonDaoImpl;
type ProductDao = ProductDaoImpl;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type PermissionDao = inventurly_dao::permission::MockPermissionDao;

#[cfg(feature = "oidc")]  
type PermissionDao = inventurly_dao_impl_sqlite::permission::PermissionDaoImpl;
type UuidService = inventurly_service_impl::uuid_service::UuidServiceImpl;
type UserService = MockUserService;

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
    inventurly_service_impl::permission::PermissionServiceImpl<PermissionServiceDependencies>;

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type SessionService = inventurly_service_impl::session::MockSessionServiceImpl;

#[cfg(feature = "oidc")]
type SessionService = inventurly_service_impl::session::SessionServiceImpl<SessionServiceDependencies>;

#[cfg(feature = "oidc")]
pub struct SessionServiceDependencies;

#[cfg(feature = "oidc")]
unsafe impl Send for SessionServiceDependencies {}

#[cfg(feature = "oidc")]
unsafe impl Sync for SessionServiceDependencies {}

#[cfg(feature = "oidc")]
impl inventurly_service_impl::session::SessionServiceDeps for SessionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type PermissionDao = PermissionDao;
}

pub struct PersonServiceDependencies;

unsafe impl Send for PersonServiceDependencies {}
unsafe impl Sync for PersonServiceDependencies {}

impl PersonServiceDeps for PersonServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type PersonDao = PersonDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type PersonService = inventurly_service_impl::person::PersonServiceImpl<PersonServiceDependencies>;

pub struct ProductServiceDependencies;

unsafe impl Send for ProductServiceDependencies {}
unsafe impl Sync for ProductServiceDependencies {}

impl ProductServiceDeps for ProductServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ProductDao = ProductDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type ProductService = inventurly_service_impl::product::ProductServiceImpl<ProductServiceDependencies>;

pub struct DuplicateDetectionServiceDependencies;

unsafe impl Send for DuplicateDetectionServiceDependencies {}
unsafe impl Sync for DuplicateDetectionServiceDependencies {}

impl DuplicateDetectionServiceDeps for DuplicateDetectionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ProductService = ProductService;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type DuplicateDetectionService = inventurly_service_impl::duplicate_detection::DuplicateDetectionServiceImpl<DuplicateDetectionServiceDependencies>;

pub struct CsvImportServiceDependencies;

unsafe impl Send for CsvImportServiceDependencies {}
unsafe impl Sync for CsvImportServiceDependencies {}

impl CsvImportServiceDeps for CsvImportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ProductService = ProductService;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type CsvImportService = inventurly_service_impl::csv_import::CsvImportServiceImpl<CsvImportServiceDependencies>;

// RestStateImpl with all services
#[derive(Clone)]
pub struct RestStateImpl {
    person_service: Arc<PersonService>,
    product_service: Arc<ProductService>,
    csv_import_service: Arc<CsvImportService>,
    duplicate_detection_service: Arc<DuplicateDetectionService>,
    permission_service: Arc<PermissionService>,
    session_service: Arc<SessionService>,
}

impl RestStateImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        // Create DAOs
        let transaction_dao = Arc::new(TransactionDao::new(pool.clone()));
        let person_dao = Arc::new(PersonDao::new(pool.clone()));
        let product_dao = Arc::new(ProductDao::new(pool.clone()));
        #[cfg(feature = "mock_auth")]
        let permission_dao = Arc::new(inventurly_dao::permission::MockPermissionDao);
        
        #[cfg(feature = "oidc")]
        let permission_dao = Arc::new(inventurly_dao_impl_sqlite::permission::PermissionDaoImpl::new(pool.clone()));
        
        // Create services
        let user_service = Arc::new(inventurly_service::user_service::MockUserService);
        let uuid_service = Arc::new(UuidService::new());
        
        // Create PermissionService using struct literal syntax
        let permission_service = Arc::new(inventurly_service_impl::permission::PermissionServiceImpl {
            permission_dao: permission_dao.clone(),
            user_service: user_service,
        });
        
        // Create PersonService using struct literal syntax
        let person_service = Arc::new(PersonServiceImpl {
            person_dao: person_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });
        
        // Create ProductService using struct literal syntax
        let product_service = Arc::new(ProductServiceImpl {
            product_dao: product_dao,
            permission_service: permission_service.clone(),
            uuid_service: uuid_service,
            transaction_dao: transaction_dao.clone(),
        });
        
        // Create DuplicateDetectionService using struct literal syntax
        let duplicate_detection_service = Arc::new(DuplicateDetectionServiceImpl {
            product_service: product_service.clone(),
            permission_service: permission_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });
        
        // Create CsvImportService using struct literal syntax
        let csv_import_service = Arc::new(CsvImportServiceImpl {
            product_service: product_service.clone(),
            permission_service: permission_service.clone(),
            transaction_dao: transaction_dao,
        });
        
        // Create SessionService
        #[cfg(feature = "mock_auth")]
        let session_service = Arc::new(inventurly_service_impl::session::MockSessionServiceImpl);
        
        #[cfg(feature = "oidc")]
        let session_service = Arc::new(inventurly_service_impl::session::SessionServiceImpl {
            permission_dao: permission_dao.clone(),
        });
        
        Self {
            person_service,
            product_service,
            csv_import_service,
            duplicate_detection_service,
            permission_service,
            session_service,
        }
    }
}

impl inventurly_rest::RestStateDef for RestStateImpl {
    type PersonService = PersonService;
    type ProductService = ProductService;
    type CsvImportService = CsvImportService;
    type DuplicateDetectionService = DuplicateDetectionService;
    type PermissionService = PermissionService;
    type SessionService = SessionService;

    fn person_service(&self) -> Arc<Self::PersonService> {
        self.person_service.clone()
    }
    
    fn product_service(&self) -> Arc<Self::ProductService> {
        self.product_service.clone()
    }
    
    fn csv_import_service(&self) -> Arc<Self::CsvImportService> {
        self.csv_import_service.clone()
    }
    
    fn duplicate_detection_service(&self) -> Arc<Self::DuplicateDetectionService> {
        self.duplicate_detection_service.clone()
    }
    
    fn permission_service(&self) -> Arc<Self::PermissionService> {
        self.permission_service.clone()
    }
    
    fn session_service(&self) -> Arc<Self::SessionService> {
        self.session_service.clone()
    }
}