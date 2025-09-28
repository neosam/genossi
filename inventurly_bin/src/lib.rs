use std::sync::Arc;

use inventurly_dao_impl_sqlite::{person::PersonDaoImpl, product::ProductDaoImpl, TransactionDaoImpl, TransactionImpl};
use inventurly_service::permission::MockContext;
use inventurly_service::user_service::MockUserService;
use inventurly_service_impl::{
    permission::PermissionServiceDeps,
    person::{PersonServiceDeps, PersonServiceImpl},
    product::{ProductServiceDeps, ProductServiceImpl},
};
use sqlx::SqlitePool;

// Type aliases for clarity
type Context = MockContext;
type Transaction = TransactionImpl;
type TransactionDao = TransactionDaoImpl;
type PersonDao = PersonDaoImpl;
type ProductDao = ProductDaoImpl;
type PermissionDao = inventurly_dao::permission::MockPermissionDao;
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

// RestStateImpl with all services
#[derive(Clone)]
pub struct RestStateImpl {
    person_service: Arc<PersonService>,
    product_service: Arc<ProductService>,
}

impl RestStateImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        // Create DAOs
        let transaction_dao = Arc::new(TransactionDao::new(pool.clone()));
        let person_dao = Arc::new(PersonDao::new(pool.clone()));
        let product_dao = Arc::new(ProductDao::new(pool.clone()));
        let permission_dao = Arc::new(inventurly_dao::permission::MockPermissionDao);
        
        // Create services
        let user_service = Arc::new(inventurly_service::user_service::MockUserService);
        let uuid_service = Arc::new(UuidService::new());
        
        // Create PermissionService using struct literal syntax
        let permission_service = Arc::new(inventurly_service_impl::permission::PermissionServiceImpl {
            permission_dao: permission_dao,
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
            permission_service: permission_service,
            uuid_service: uuid_service,
            transaction_dao: transaction_dao,
        });
        
        Self {
            person_service,
            product_service,
        }
    }
}

impl inventurly_rest::RestStateDef for RestStateImpl {
    type PersonService = PersonService;
    type ProductService = ProductService;

    fn person_service(&self) -> Arc<Self::PersonService> {
        self.person_service.clone()
    }
    
    fn product_service(&self) -> Arc<Self::ProductService> {
        self.product_service.clone()
    }
}