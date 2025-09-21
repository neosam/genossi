use std::sync::Arc;

use inventurly_dao_impl_sqlite::{person::PersonDaoImpl, TransactionDaoImpl, TransactionImpl};
use inventurly_service::permission::MockContext;
use inventurly_service::user_service::MockUserService;
use inventurly_service_impl::{
    permission::PermissionServiceDeps,
    person::{PersonServiceDeps, PersonServiceImpl},
};
use sqlx::SqlitePool;

// Type aliases for clarity
type Context = MockContext;
type Transaction = TransactionImpl;
type TransactionDao = TransactionDaoImpl;
type PersonDao = PersonDaoImpl;
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

// RestStateImpl with all services
#[derive(Clone)]
pub struct RestStateImpl {
    person_service: Arc<PersonService>,
}

impl RestStateImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        // Create DAOs
        let transaction_dao = Arc::new(TransactionDao::new(pool.clone()));
        let person_dao = Arc::new(PersonDao::new(pool.clone()));
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
            person_dao: person_dao,
            permission_service: permission_service,
            uuid_service: uuid_service,
            transaction_dao: transaction_dao,
        });
        
        Self {
            person_service,
        }
    }
}

impl inventurly_rest::RestStateDef for RestStateImpl {
    type PersonService = PersonService;

    fn person_service(&self) -> Arc<Self::PersonService> {
        self.person_service.clone()
    }
}