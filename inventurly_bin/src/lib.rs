use std::sync::Arc;

use inventurly_dao_impl_sqlite::{
    container::SqliteContainerDao, container_rack::ContainerRackDaoImpl, inventur::InventurDaoImpl,
    inventur_custom_entry::InventurCustomEntryDaoImpl,
    inventur_measurement::InventurMeasurementDaoImpl, person::PersonDaoImpl,
    product::ProductDaoImpl, product_rack::ProductRackDaoImpl, rack::RackDaoImpl,
    TransactionDaoImpl, TransactionImpl,
};
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use inventurly_service::permission::MockContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use inventurly_service::user_service::MockUserService;
#[cfg(feature = "oidc")]
use inventurly_service::auth_types::AuthenticatedContext;
use inventurly_service_impl::{
    container::{ContainerServiceDeps, ContainerServiceImpl},
    container_rack::ContainerRackServiceImpl,
    csv_import::{CsvImportServiceDeps, CsvImportServiceImpl},
    deposit_ean_import::{DepositEanImportServiceDeps, DepositEanImportServiceImpl},
    duplicate_detection::{DuplicateDetectionServiceDeps, DuplicateDetectionServiceImpl},
    inventur::{InventurServiceDeps, InventurServiceImpl},
    inventur_custom_entry::{InventurCustomEntryServiceDeps, InventurCustomEntryServiceImpl},
    inventur_measurement::{InventurMeasurementServiceDeps, InventurMeasurementServiceImpl},
    inventur_report::{InventurReportServiceDeps, InventurReportServiceImpl},
    permission::PermissionServiceDeps,
    person::{PersonServiceDeps, PersonServiceImpl},
    price_import::{PriceImportServiceDeps, PriceImportServiceImpl},
    product::{ProductServiceDeps, ProductServiceImpl},
    product_rack::ProductRackServiceImpl,
    rack::{RackServiceDeps, RackServiceImpl},
};
use sqlx::SqlitePool;

// Type aliases for clarity
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type Context = MockContext;
#[cfg(feature = "oidc")]
type Context = AuthenticatedContext;
type Transaction = TransactionImpl;
type TransactionDao = TransactionDaoImpl;
type PersonDao = PersonDaoImpl;
type ProductDao = ProductDaoImpl;
type RackDao = RackDaoImpl;
type ProductRackDao = ProductRackDaoImpl;
type ContainerDao = SqliteContainerDao;
type ContainerRackDao = ContainerRackDaoImpl;
type InventurDao = InventurDaoImpl;
type InventurMeasurementDao = InventurMeasurementDaoImpl;
type InventurCustomEntryDao = InventurCustomEntryDaoImpl;
// Always use real SQLite permission DAO regardless of auth mode
type PermissionDao = inventurly_dao_impl_sqlite::permission::PermissionDaoImpl;
type UuidService = inventurly_service_impl::uuid_service::UuidServiceImpl;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type UserService = MockUserService;
#[cfg(feature = "oidc")]
type UserService = inventurly_service_impl::user_service::AuthContextUserService;

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
type SessionService =
    inventurly_service_impl::session::SessionServiceImpl<SessionServiceDependencies>;

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

pub struct RackServiceDependencies;

unsafe impl Send for RackServiceDependencies {}
unsafe impl Sync for RackServiceDependencies {}

impl RackServiceDeps for RackServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type RackDao = RackDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type RackService = inventurly_service_impl::rack::RackServiceImpl<RackServiceDependencies>;

pub struct ProductRackServiceDependencies;

unsafe impl Send for ProductRackServiceDependencies {}
unsafe impl Sync for ProductRackServiceDependencies {}

impl inventurly_service_impl::product_rack::ProductRackServiceDependencies
    for ProductRackServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type ProductRackDao = ProductRackDao;
    type ProductDao = ProductDao;
    type RackDao = RackDao;
    type TransactionDao = TransactionDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
}

type ProductRackService =
    inventurly_service_impl::product_rack::ProductRackServiceImpl<ProductRackServiceDependencies>;

pub struct ContainerServiceDependencies;

unsafe impl Send for ContainerServiceDependencies {}
unsafe impl Sync for ContainerServiceDependencies {}

impl ContainerServiceDeps for ContainerServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ContainerDao = ContainerDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type ContainerService =
    inventurly_service_impl::container::ContainerServiceImpl<ContainerServiceDependencies>;

pub struct ContainerRackServiceDependencies;

unsafe impl Send for ContainerRackServiceDependencies {}
unsafe impl Sync for ContainerRackServiceDependencies {}

impl inventurly_service_impl::container_rack::ContainerRackServiceDependencies
    for ContainerRackServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type ContainerRackDao = ContainerRackDao;
    type ContainerDao = ContainerDao;
    type RackDao = RackDao;
    type TransactionDao = TransactionDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
}

type ContainerRackService =
    inventurly_service_impl::container_rack::ContainerRackServiceImpl<ContainerRackServiceDependencies>;

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

type ProductService =
    inventurly_service_impl::product::ProductServiceImpl<ProductServiceDependencies>;

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

type DuplicateDetectionService =
    inventurly_service_impl::duplicate_detection::DuplicateDetectionServiceImpl<
        DuplicateDetectionServiceDependencies,
    >;

pub struct CsvImportServiceDependencies;

unsafe impl Send for CsvImportServiceDependencies {}
unsafe impl Sync for CsvImportServiceDependencies {}

impl CsvImportServiceDeps for CsvImportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ProductService = ProductService;
    type ProductDao = ProductDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type CsvImportService =
    inventurly_service_impl::csv_import::CsvImportServiceImpl<CsvImportServiceDependencies>;

pub struct PriceImportServiceDependencies;

unsafe impl Send for PriceImportServiceDependencies {}
unsafe impl Sync for PriceImportServiceDependencies {}

impl PriceImportServiceDeps for PriceImportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ProductService = ProductService;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type PriceImportService =
    inventurly_service_impl::price_import::PriceImportServiceImpl<PriceImportServiceDependencies>;

pub struct DepositEanImportServiceDependencies;

unsafe impl Send for DepositEanImportServiceDependencies {}
unsafe impl Sync for DepositEanImportServiceDependencies {}

impl DepositEanImportServiceDeps for DepositEanImportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ProductService = ProductService;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type DepositEanImportService =
    inventurly_service_impl::deposit_ean_import::DepositEanImportServiceImpl<DepositEanImportServiceDependencies>;

pub struct InventurServiceDependencies;

unsafe impl Send for InventurServiceDependencies {}
unsafe impl Sync for InventurServiceDependencies {}

impl InventurServiceDeps for InventurServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type InventurDao = InventurDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type InventurService =
    inventurly_service_impl::inventur::InventurServiceImpl<InventurServiceDependencies>;

pub struct InventurMeasurementServiceDependencies;

unsafe impl Send for InventurMeasurementServiceDependencies {}
unsafe impl Sync for InventurMeasurementServiceDependencies {}

impl InventurMeasurementServiceDeps for InventurMeasurementServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type InventurMeasurementDao = InventurMeasurementDao;
    type InventurDao = InventurDao;
    type ProductDao = ProductDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type InventurMeasurementService = inventurly_service_impl::inventur_measurement::InventurMeasurementServiceImpl<
    InventurMeasurementServiceDependencies,
>;

pub struct InventurCustomEntryServiceDependencies;

unsafe impl Send for InventurCustomEntryServiceDependencies {}
unsafe impl Sync for InventurCustomEntryServiceDependencies {}

impl InventurCustomEntryServiceDeps for InventurCustomEntryServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type InventurCustomEntryDao = InventurCustomEntryDao;
    type InventurDao = InventurDao;
    type ProductDao = ProductDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type InventurCustomEntryService = inventurly_service_impl::inventur_custom_entry::InventurCustomEntryServiceImpl<
    InventurCustomEntryServiceDependencies,
>;

pub struct InventurReportServiceDependencies;

unsafe impl Send for InventurReportServiceDependencies {}
unsafe impl Sync for InventurReportServiceDependencies {}

impl InventurReportServiceDeps for InventurReportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type InventurMeasurementDao = InventurMeasurementDao;
    type InventurCustomEntryDao = InventurCustomEntryDao;
    type ProductDao = ProductDao;
    type RackDao = RackDao;
    type ContainerDao = ContainerDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type InventurReportService = inventurly_service_impl::inventur_report::InventurReportServiceImpl<
    InventurReportServiceDependencies,
>;

// RestStateImpl with all services
#[derive(Clone)]
pub struct RestStateImpl {
    person_service: Arc<PersonService>,
    product_service: Arc<ProductService>,
    rack_service: Arc<RackService>,
    product_rack_service: Arc<ProductRackService>,
    container_service: Arc<ContainerService>,
    container_rack_service: Arc<ContainerRackService>,
    inventur_service: Arc<InventurService>,
    inventur_measurement_service: Arc<InventurMeasurementService>,
    inventur_custom_entry_service: Arc<InventurCustomEntryService>,
    inventur_report_service: Arc<InventurReportService>,
    csv_import_service: Arc<CsvImportService>,
    price_import_service: Arc<PriceImportService>,
    deposit_ean_import_service: Arc<DepositEanImportService>,
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
        let rack_dao = Arc::new(RackDao::new(pool.clone()));
        let product_rack_dao = Arc::new(ProductRackDao::new(pool.clone()));
        let container_dao = Arc::new(ContainerDao::new(pool.as_ref().clone()));
        let container_rack_dao = Arc::new(ContainerRackDao::new(pool.clone()));
        let inventur_dao = Arc::new(InventurDao::new(pool.clone()));
        let inventur_measurement_dao = Arc::new(InventurMeasurementDao::new(pool.clone()));
        let inventur_custom_entry_dao = Arc::new(InventurCustomEntryDao::new(pool.clone()));
        // Always use real SQLite permission DAO regardless of auth mode
        // Mock auth should only mock user identity, not the actual data
        let permission_dao =
            Arc::new(inventurly_dao_impl_sqlite::permission::PermissionDaoImpl::new(pool.clone()));

        // Create services
        #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
        let user_service = Arc::new(MockUserService);
        #[cfg(feature = "oidc")]
        let user_service = Arc::new(inventurly_service_impl::user_service::AuthContextUserService);
        let uuid_service = Arc::new(UuidService::new());

        // Create PermissionService using struct literal syntax
        let permission_service =
            Arc::new(inventurly_service_impl::permission::PermissionServiceImpl {
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

        // Create RackService using struct literal syntax
        let rack_service = Arc::new(RackServiceImpl {
            rack_dao: rack_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create ProductService using struct literal syntax
        let product_service = Arc::new(ProductServiceImpl {
            product_dao: product_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create ProductRackService using struct literal syntax
        let product_rack_service = Arc::new(ProductRackServiceImpl {
            product_rack_dao: product_rack_dao,
            product_dao: product_dao.clone(),
            rack_dao: rack_dao.clone(),
            transaction_dao: transaction_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
        });

        // Create ContainerService using struct literal syntax
        let container_service = Arc::new(ContainerServiceImpl {
            container_dao: container_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create ContainerRackService using struct literal syntax
        let container_rack_service = Arc::new(ContainerRackServiceImpl {
            container_rack_dao: container_rack_dao,
            container_dao: container_dao.clone(),
            rack_dao: rack_dao.clone(),
            transaction_dao: transaction_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
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
            product_dao: product_dao.clone(),
            permission_service: permission_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create PriceImportService using struct literal syntax
        let price_import_service = Arc::new(PriceImportServiceImpl {
            product_service: product_service.clone(),
            permission_service: permission_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create DepositEanImportService using struct literal syntax
        let deposit_ean_import_service = Arc::new(DepositEanImportServiceImpl {
            product_service: product_service.clone(),
            permission_service: permission_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create InventurService using struct literal syntax
        let inventur_service = Arc::new(InventurServiceImpl {
            inventur_dao: inventur_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        // Create InventurMeasurementService using struct literal syntax
        let inventur_measurement_service = Arc::new(InventurMeasurementServiceImpl {
            inventur_measurement_dao: inventur_measurement_dao.clone(),
            inventur_dao: inventur_dao.clone(),
            product_dao: product_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service.clone(),
            transaction_dao: transaction_dao.clone(),
        });

        let inventur_custom_entry_service = Arc::new(InventurCustomEntryServiceImpl {
            inventur_custom_entry_dao: inventur_custom_entry_dao.clone(),
            inventur_dao: inventur_dao,
            product_dao: product_dao.clone(),
            permission_service: permission_service.clone(),
            uuid_service: uuid_service,
            transaction_dao: transaction_dao.clone(),
        });

        let inventur_report_service = Arc::new(InventurReportServiceImpl {
            inventur_measurement_dao: inventur_measurement_dao.clone(),
            inventur_custom_entry_dao: inventur_custom_entry_dao,
            product_dao: product_dao,
            rack_dao: rack_dao,
            container_dao: container_dao.clone(),
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
            rack_service,
            product_rack_service,
            container_service,
            container_rack_service,
            inventur_service,
            inventur_measurement_service,
            inventur_custom_entry_service,
            inventur_report_service,
            csv_import_service,
            price_import_service,
            deposit_ean_import_service,
            duplicate_detection_service,
            permission_service,
            session_service,
        }
    }
}

impl inventurly_rest::RestStateDef for RestStateImpl {
    type PersonService = PersonService;
    type ProductService = ProductService;
    type RackService = RackService;
    type ProductRackService = ProductRackService;
    type ContainerService = ContainerService;
    type ContainerRackService = ContainerRackService;
    type InventurService = InventurService;
    type InventurMeasurementService = InventurMeasurementService;
    type InventurCustomEntryService = InventurCustomEntryService;
    type InventurReportService = InventurReportService;
    type CsvImportService = CsvImportService;
    type PriceImportService = PriceImportService;
    type DepositEanImportService = DepositEanImportService;
    type DuplicateDetectionService = DuplicateDetectionService;
    type PermissionService = PermissionService;
    type SessionService = SessionService;

    fn person_service(&self) -> Arc<Self::PersonService> {
        self.person_service.clone()
    }

    fn product_service(&self) -> Arc<Self::ProductService> {
        self.product_service.clone()
    }

    fn rack_service(&self) -> Arc<Self::RackService> {
        self.rack_service.clone()
    }

    fn product_rack_service(&self) -> Arc<Self::ProductRackService> {
        self.product_rack_service.clone()
    }

    fn container_service(&self) -> Arc<Self::ContainerService> {
        self.container_service.clone()
    }

    fn container_rack_service(&self) -> Arc<Self::ContainerRackService> {
        self.container_rack_service.clone()
    }

    fn inventur_service(&self) -> Arc<Self::InventurService> {
        self.inventur_service.clone()
    }

    fn inventur_measurement_service(&self) -> Arc<Self::InventurMeasurementService> {
        self.inventur_measurement_service.clone()
    }

    fn inventur_custom_entry_service(&self) -> Arc<Self::InventurCustomEntryService> {
        self.inventur_custom_entry_service.clone()
    }

    fn inventur_report_service(&self) -> Arc<Self::InventurReportService> {
        self.inventur_report_service.clone()
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

    fn price_import_service(&self) -> Arc<Self::PriceImportService> {
        self.price_import_service.clone()
    }

    fn deposit_ean_import_service(&self) -> Arc<Self::DepositEanImportService> {
        self.deposit_ean_import_service.clone()
    }

    fn session_service(&self) -> Arc<Self::SessionService> {
        self.session_service.clone()
    }
}
