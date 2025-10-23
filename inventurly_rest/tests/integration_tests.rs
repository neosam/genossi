use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use inventurly_rest::RestStateDef;
use inventurly_rest_types::{ContainerTO, PersonTO, ProductRackTO, RackTO};
use inventurly_service::container::{Container, ContainerService};
use inventurly_service::csv_import::{
    CsvImportResult, CsvImportService, CsvProductRow, ImportAction,
};
use inventurly_service::duplicate_detection::{
    DuplicateDetectionConfig, DuplicateDetectionResult, DuplicateDetectionService, DuplicateMatch,
};
use inventurly_service::permission::PermissionService;
use inventurly_service::permission::{Authentication, MockContext};
use inventurly_service::person::{Person, PersonService};
use inventurly_service::product::{Product, ProductService};
use inventurly_service::product_rack::{ProductRack, ProductRackService};
use inventurly_service::rack::{Rack, RackService};
use inventurly_service::session::SessionService;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone)]
struct TestRestState {
    person_service: Arc<MockPersonService>,
    product_service: Arc<MockProductService>,
    rack_service: Arc<MockRackService>,
    product_rack_service: Arc<MockProductRackService>,
    csv_import_service: Arc<MockCsvImportService>,
    duplicate_detection_service: Arc<MockDuplicateDetectionService>,
    permission_service: Arc<MockPermissionService>,
    session_service: Arc<MockSessionService>,
    container_service: Arc<MockContainerService>,
}

impl RestStateDef for TestRestState {
    type PersonService = MockPersonService;
    type ProductService = MockProductService;
    type RackService = MockRackService;
    type ProductRackService = MockProductRackService;
    type CsvImportService = MockCsvImportService;
    type DuplicateDetectionService = MockDuplicateDetectionService;
    type PermissionService = MockPermissionService;
    type SessionService = MockSessionService;
    type ContainerService = MockContainerService;

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

    fn container_service(&self) -> Arc<Self::ContainerService> {
        self.container_service.clone()
    }
}

#[derive(Clone)]
struct MockPersonService;

#[async_trait::async_trait]
impl PersonService for MockPersonService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn get_all(
        &self,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[Person]>, inventurly_service::ServiceError> {
        let person = Person {
            id: Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap(),
            name: Arc::from("John Doe"),
            age: 30,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::parse_str("456e7890-e12b-34c5-a678-901234567890").unwrap(),
        };
        Ok(Arc::from([person]))
    }

    async fn get(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Person, inventurly_service::ServiceError> {
        if id == Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap() {
            Ok(Person {
                id,
                name: Arc::from("John Doe"),
                age: 30,
                created: time::PrimitiveDateTime::new(
                    time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                    time::Time::from_hms(0, 0, 0).unwrap(),
                ),
                deleted: None,
                version: Uuid::parse_str("456e7890-e12b-34c5-a678-901234567890").unwrap(),
            })
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(id))
        }
    }

    async fn create(
        &self,
        person: &Person,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Person, inventurly_service::ServiceError> {
        Ok(Person {
            id: Uuid::new_v4(),
            name: person.name.clone(),
            age: person.age,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        })
    }

    async fn update(
        &self,
        person: &Person,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Person, inventurly_service::ServiceError> {
        if person.id == Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap() {
            Ok(Person {
                id: person.id,
                name: person.name.clone(),
                age: person.age,
                created: person.created,
                deleted: person.deleted,
                version: Uuid::new_v4(),
            })
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(person.id))
        }
    }

    async fn delete(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<(), inventurly_service::ServiceError> {
        if id == Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap() {
            Ok(())
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(id))
        }
    }
}

#[derive(Clone)]
struct MockProductService;

#[async_trait::async_trait]
impl ProductService for MockProductService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn get_all(
        &self,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[Product]>, inventurly_service::ServiceError> {
        Ok(Arc::from([]))
    }

    async fn get_by_ean(
        &self,
        _ean: &str,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Product, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::EntityNotFound(Uuid::nil()))
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Product, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::EntityNotFound(id))
    }

    async fn create(
        &self,
        _product: &Product,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Product, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::InternalError(Arc::from(
            "Not implemented",
        )))
    }

    async fn update(
        &self,
        _product: &Product,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Product, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::InternalError(Arc::from(
            "Not implemented",
        )))
    }

    async fn delete(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::EntityNotFound(id))
    }

    async fn search(
        &self,
        _query: &str,
        _limit: Option<usize>,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[Product]>, inventurly_service::ServiceError> {
        Ok(Arc::from([]))
    }
}

#[derive(Clone)]
struct MockRackService;

#[async_trait::async_trait]
impl RackService for MockRackService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn get_all(
        &self,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[Rack]>, inventurly_service::ServiceError> {
        let rack = Rack {
            id: Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap(),
            name: Arc::from("Storage Rack A"),
            description: Arc::from("Primary storage rack for inventory"),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::parse_str("b2c3d4e5-f6a7-8901-bcde-f23456789012").unwrap(),
        };
        Ok(Arc::from([rack]))
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Option<Rack>, inventurly_service::ServiceError> {
        if id == Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap() {
            Ok(Some(Rack {
                id,
                name: Arc::from("Storage Rack A"),
                description: Arc::from("Primary storage rack for inventory"),
                created: time::PrimitiveDateTime::new(
                    time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                    time::Time::from_hms(0, 0, 0).unwrap(),
                ),
                deleted: None,
                version: Uuid::parse_str("b2c3d4e5-f6a7-8901-bcde-f23456789012").unwrap(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn create(
        &self,
        rack: &Rack,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Rack, inventurly_service::ServiceError> {
        Ok(Rack {
            id: Uuid::new_v4(),
            name: rack.name.clone(),
            description: rack.description.clone(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        })
    }

    async fn update(
        &self,
        rack: &Rack,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Rack, inventurly_service::ServiceError> {
        if rack.id == Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap() {
            Ok(Rack {
                id: rack.id,
                name: rack.name.clone(),
                description: rack.description.clone(),
                created: rack.created,
                deleted: rack.deleted,
                version: Uuid::new_v4(),
            })
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(rack.id))
        }
    }

    async fn delete(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<(), inventurly_service::ServiceError> {
        if id == Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap() {
            Ok(())
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(id))
        }
    }
}

#[derive(Clone)]
struct MockProductRackService;

#[async_trait::async_trait]
impl ProductRackService for MockProductRackService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn add_product_to_rack(
        &self,
        product_id: Uuid,
        rack_id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<ProductRack, inventurly_service::ServiceError> {
        Ok(ProductRack {
            product_id,
            rack_id,
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        })
    }

    async fn remove_product_from_rack(
        &self,
        _product_id: Uuid,
        _rack_id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn get_racks_for_product(
        &self,
        _product_id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, inventurly_service::ServiceError> {
        Ok(Arc::from([]))
    }

    async fn get_products_in_rack(
        &self,
        _rack_id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, inventurly_service::ServiceError> {
        Ok(Arc::from([]))
    }

    async fn get_product_rack_relationship(
        &self,
        _product_id: Uuid,
        _rack_id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Option<ProductRack>, inventurly_service::ServiceError> {
        Ok(None)
    }

    async fn get_all_relationships(
        &self,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[ProductRack]>, inventurly_service::ServiceError> {
        Ok(Arc::from([]))
    }
}

#[derive(Clone)]
struct MockCsvImportService;

#[async_trait::async_trait]
impl CsvImportService for MockCsvImportService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn import_products_csv(
        &self,
        _csv_content: &str,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<CsvImportResult, inventurly_service::ServiceError> {
        Ok(CsvImportResult {
            total_rows: 0,
            created: 0,
            updated: 0,
            errors: vec![],
        })
    }

    async fn import_product_row(
        &self,
        _row: CsvProductRow,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<ImportAction, inventurly_service::ServiceError> {
        Ok(ImportAction::Created)
    }
}

#[derive(Clone)]
struct MockDuplicateDetectionService;

#[async_trait::async_trait]
impl DuplicateDetectionService for MockDuplicateDetectionService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn find_duplicates(
        &self,
        _product: &Product,
        _config: Option<DuplicateDetectionConfig>,
        _context: Authentication<Self::Context>,
        _tx: Option<Self::Transaction>,
    ) -> Result<DuplicateDetectionResult, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::InternalError(Arc::from(
            "Not implemented",
        )))
    }

    async fn find_all_duplicates(
        &self,
        _config: Option<DuplicateDetectionConfig>,
        _context: Authentication<Self::Context>,
        _tx: Option<Self::Transaction>,
    ) -> Result<Vec<DuplicateDetectionResult>, inventurly_service::ServiceError> {
        Ok(vec![])
    }

    async fn check_potential_duplicate(
        &self,
        _name: &str,
        _sales_unit: &str,
        _requires_weighing: bool,
        _config: Option<DuplicateDetectionConfig>,
        _context: Authentication<Self::Context>,
        _tx: Option<Self::Transaction>,
    ) -> Result<Vec<DuplicateMatch>, inventurly_service::ServiceError> {
        Ok(vec![])
    }
}

#[derive(Clone)]
struct MockPermissionService;

#[async_trait::async_trait]
impl PermissionService for MockPermissionService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;

    async fn check_permission(
        &self,
        _privilege: &str,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(()) // Always allow in tests
    }

    async fn get_all_users(
        &self,
        _context: Authentication<Self::Context>,
    ) -> Result<
        Arc<[inventurly_service::auth_types::UserResponseTO]>,
        inventurly_service::ServiceError,
    > {
        Ok(Arc::new([]))
    }

    async fn create_user(
        &self,
        _user: inventurly_service::auth_types::UserTO,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn delete_user(
        &self,
        _username: String,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn get_all_roles(
        &self,
        _context: Authentication<Self::Context>,
    ) -> Result<
        Arc<[inventurly_service::auth_types::RoleResponseTO]>,
        inventurly_service::ServiceError,
    > {
        Ok(Arc::new([]))
    }

    async fn create_role(
        &self,
        _role: inventurly_service::auth_types::RoleTO,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn delete_role(
        &self,
        _role_name: String,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn get_all_privileges(
        &self,
        _context: Authentication<Self::Context>,
    ) -> Result<
        Arc<[inventurly_service::auth_types::PrivilegeResponseTO]>,
        inventurly_service::ServiceError,
    > {
        Ok(Arc::new([]))
    }

    async fn create_privilege(
        &self,
        _privilege: inventurly_service::auth_types::PrivilegeTO,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn delete_privilege(
        &self,
        _privilege_name: String,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn assign_user_role(
        &self,
        _user_role: inventurly_service::auth_types::UserRole,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn remove_user_role(
        &self,
        _user_role: inventurly_service::auth_types::UserRole,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn get_user_roles(
        &self,
        _username: String,
        _context: Authentication<Self::Context>,
    ) -> Result<
        Arc<[inventurly_service::auth_types::RoleResponseTO]>,
        inventurly_service::ServiceError,
    > {
        Ok(Arc::new([]))
    }

    async fn assign_role_privilege(
        &self,
        _role_privilege: inventurly_service::auth_types::RolePrivilege,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn remove_role_privilege(
        &self,
        _role_privilege: inventurly_service::auth_types::RolePrivilege,
        _context: Authentication<Self::Context>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn get_role_privileges(
        &self,
        _role_name: String,
        _context: Authentication<Self::Context>,
    ) -> Result<
        Arc<[inventurly_service::auth_types::PrivilegeResponseTO]>,
        inventurly_service::ServiceError,
    > {
        Ok(Arc::new([]))
    }

    async fn get_user_privileges(
        &self,
        _username: String,
        _context: Authentication<Self::Context>,
    ) -> Result<
        Arc<[inventurly_service::auth_types::PrivilegeResponseTO]>,
        inventurly_service::ServiceError,
    > {
        Ok(Arc::new([]))
    }

    async fn current_user_id(
        &self,
        _context: Authentication<Self::Context>,
    ) -> Result<Option<String>, inventurly_service::ServiceError> {
        Ok(Some("testuser".to_string()))
    }
}

#[derive(Clone)]
struct MockSessionService;

#[async_trait::async_trait]
impl SessionService for MockSessionService {
    async fn extract_auth_context(
        &self,
        _session_id: Option<String>,
    ) -> Result<Option<inventurly_service::auth_types::AuthContext>, inventurly_service::ServiceError>
    {
        Ok(Some(inventurly_service::auth_types::AuthContext::Mock(
            inventurly_service::auth_types::MockContext::default(),
        )))
    }

    async fn create_session(
        &self,
        _user_id: &str,
        _expires_at: i64,
    ) -> Result<inventurly_service::auth_types::UserSession, inventurly_service::ServiceError> {
        Ok(inventurly_service::auth_types::UserSession {
            session_id: "test-session".into(),
            user_id: "testuser".into(),
            expires_at: 9999999999,
            created_at: 1000000000,
        })
    }

    async fn verify_user_session(
        &self,
        _session_id: &str,
    ) -> Result<Option<inventurly_service::auth_types::UserSession>, inventurly_service::ServiceError>
    {
        Ok(Some(inventurly_service::auth_types::UserSession {
            session_id: "test-session".into(),
            user_id: "testuser".into(),
            expires_at: 9999999999,
            created_at: 1000000000,
        }))
    }

    async fn invalidate_session(
        &self,
        _session_id: &str,
    ) -> Result<(), inventurly_service::ServiceError> {
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, inventurly_service::ServiceError> {
        Ok(0)
    }
}

#[derive(Clone)]
struct MockContainerService;

#[async_trait::async_trait]
impl ContainerService for MockContainerService {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    type Context = MockContext;
    #[cfg(feature = "oidc")]
    type Context = inventurly_service::auth_types::AuthenticatedContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn get_all(
        &self,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[Container]>, inventurly_service::ServiceError> {
        let container = Container {
            id: Uuid::parse_str("c1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap(),
            name: Arc::from("Test Container"),
            weight_grams: 500,
            description: Arc::from("A test container for storage"),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::parse_str("456e7890-e12b-34c5-a678-901234567890").unwrap(),
        };
        Ok(Arc::from([container]))
    }

    async fn get_by_id(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Container, inventurly_service::ServiceError> {
        if id == Uuid::parse_str("c1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap() {
            Ok(Container {
                id,
                name: Arc::from("Test Container"),
                weight_grams: 500,
                description: Arc::from("A test container for storage"),
                created: time::PrimitiveDateTime::new(
                    time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                    time::Time::from_hms(0, 0, 0).unwrap(),
                ),
                deleted: None,
                version: Uuid::parse_str("456e7890-e12b-34c5-a678-901234567890").unwrap(),
            })
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(id))
        }
    }

    async fn get_by_name(
        &self,
        name: &str,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Container, inventurly_service::ServiceError> {
        if name == "Test Container" {
            Ok(Container {
                id: Uuid::parse_str("c1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap(),
                name: Arc::from("Test Container"),
                weight_grams: 500,
                description: Arc::from("A test container for storage"),
                created: time::PrimitiveDateTime::new(
                    time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                    time::Time::from_hms(0, 0, 0).unwrap(),
                ),
                deleted: None,
                version: Uuid::parse_str("456e7890-e12b-34c5-a678-901234567890").unwrap(),
            })
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(Uuid::nil()))
        }
    }

    async fn create(
        &self,
        container: &Container,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Container, inventurly_service::ServiceError> {
        Ok(Container {
            id: Uuid::new_v4(),
            name: container.name.clone(),
            weight_grams: container.weight_grams,
            description: container.description.clone(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: Uuid::new_v4(),
        })
    }

    async fn update(
        &self,
        container: &Container,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Container, inventurly_service::ServiceError> {
        if container.id == Uuid::parse_str("c1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap() {
            Ok(Container {
                id: container.id,
                name: container.name.clone(),
                weight_grams: container.weight_grams,
                description: container.description.clone(),
                created: container.created,
                deleted: container.deleted,
                version: Uuid::new_v4(),
            })
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(container.id))
        }
    }

    async fn delete(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<(), inventurly_service::ServiceError> {
        if id == Uuid::parse_str("c1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap() {
            Ok(())
        } else {
            Err(inventurly_service::ServiceError::EntityNotFound(id))
        }
    }

    async fn search(
        &self,
        _query: &str,
        _limit: Option<usize>,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Arc<[Container]>, inventurly_service::ServiceError> {
        // Return an empty result for search
        Ok(Arc::from([]))
    }
}

fn create_test_app() -> axum::Router {
    let rest_state = TestRestState {
        person_service: Arc::new(MockPersonService),
        product_service: Arc::new(MockProductService),
        rack_service: Arc::new(MockRackService),
        product_rack_service: Arc::new(MockProductRackService),
        csv_import_service: Arc::new(MockCsvImportService),
        duplicate_detection_service: Arc::new(MockDuplicateDetectionService),
        permission_service: Arc::new(MockPermissionService),
        session_service: Arc::new(MockSessionService),
        container_service: Arc::new(MockContainerService),
    };

    axum::Router::new()
        .nest("/persons", inventurly_rest::person::generate_route())
        .nest("/racks", inventurly_rest::rack::generate_route())
        .nest("/containers", inventurly_rest::container::generate_route())
        .nest(
            "/product-racks",
            inventurly_rest::product_rack::generate_route(),
        )
        .with_state(rest_state)
        .layer(axum::middleware::from_fn(
            |mut req: Request<Body>, next: axum::middleware::Next| async move {
                req.extensions_mut()
                    .insert(inventurly_rest::Context::default());
                next.run(req).await
            },
        ))
}

#[tokio::test]
async fn test_get_all_persons() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/persons")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let persons: Vec<PersonTO> = serde_json::from_slice(&body).unwrap();

    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0].name, "John Doe");
    assert_eq!(persons[0].age, 30);
}

#[tokio::test]
async fn test_get_person_by_id() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/persons/123e4567-e89b-12d3-a456-426614174000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let person: PersonTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(person.name, "John Doe");
    assert_eq!(person.age, 30);
    assert_eq!(
        person.id,
        Some(Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap())
    );
}

#[tokio::test]
async fn test_get_person_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/persons/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_person() {
    let app = create_test_app();

    let new_person = json!({
        "name": "Jane Smith",
        "age": 25
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/persons")
                .header("Content-Type", "application/json")
                .body(Body::from(new_person.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let person: PersonTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(person.name, "Jane Smith");
    assert_eq!(person.age, 25);
    assert!(person.id.is_some());
}

#[tokio::test]
async fn test_update_person() {
    let app = create_test_app();

    let updated_person = json!({
        "name": "John Updated",
        "age": 35
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/persons/123e4567-e89b-12d3-a456-426614174000")
                .header("Content-Type", "application/json")
                .body(Body::from(updated_person.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let person: PersonTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(person.name, "John Updated");
    assert_eq!(person.age, 35);
    assert_eq!(
        person.id,
        Some(Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap())
    );
}

#[tokio::test]
async fn test_update_person_not_found() {
    let app = create_test_app();

    let updated_person = json!({
        "name": "NonExistent",
        "age": 99
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/persons/00000000-0000-0000-0000-000000000000")
                .header("Content-Type", "application/json")
                .body(Body::from(updated_person.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_person() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/persons/123e4567-e89b-12d3-a456-426614174000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_person_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/persons/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Rack tests
#[tokio::test]
async fn test_get_all_racks() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/racks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let racks: Vec<RackTO> = serde_json::from_slice(&body).unwrap();

    assert_eq!(racks.len(), 1);
    assert_eq!(racks[0].name, "Storage Rack A");
    assert_eq!(racks[0].description, "Primary storage rack for inventory");
}

#[tokio::test]
async fn test_get_rack_by_id() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/racks/a1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let rack: RackTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(rack.name, "Storage Rack A");
    assert_eq!(rack.description, "Primary storage rack for inventory");
    assert_eq!(
        rack.id,
        Some(Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap())
    );
}

#[tokio::test]
async fn test_get_rack_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/racks/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_rack() {
    let app = create_test_app();

    let new_rack = json!({
        "name": "Storage Rack B",
        "description": "Secondary storage rack"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/racks")
                .header("Content-Type", "application/json")
                .body(Body::from(new_rack.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let rack: RackTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(rack.name, "Storage Rack B");
    assert_eq!(rack.description, "Secondary storage rack");
    assert!(rack.id.is_some());
}

#[tokio::test]
async fn test_update_rack() {
    let app = create_test_app();

    let updated_rack = json!({
        "name": "Updated Rack A",
        "description": "Updated description for rack A"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/racks/a1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .header("Content-Type", "application/json")
                .body(Body::from(updated_rack.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let rack: RackTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(rack.name, "Updated Rack A");
    assert_eq!(rack.description, "Updated description for rack A");
    assert_eq!(
        rack.id,
        Some(Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap())
    );
}

#[tokio::test]
async fn test_update_rack_not_found() {
    let app = create_test_app();

    let updated_rack = json!({
        "name": "NonExistent",
        "description": "This rack does not exist"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/racks/00000000-0000-0000-0000-000000000000")
                .header("Content-Type", "application/json")
                .body(Body::from(updated_rack.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_rack() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/racks/a1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_rack_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/racks/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Product-Rack tests
#[tokio::test]
async fn test_add_product_to_rack() {
    let app = create_test_app();

    let request_body = json!({
        "product_id": "123e4567-e89b-12d3-a456-426614174000",
        "rack_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/product-racks")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let product_rack: ProductRackTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        product_rack.product_id,
        Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap()
    );
    assert_eq!(
        product_rack.rack_id,
        Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()
    );
}

#[tokio::test]
async fn test_remove_product_from_rack() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/product-racks/123e4567-e89b-12d3-a456-426614174000/a1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_get_product_rack_relationship() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/product-racks/123e4567-e89b-12d3-a456-426614174000/a1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Mock service returns None, so we expect 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_racks_for_product() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/product-racks/product/123e4567-e89b-12d3-a456-426614174000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let product_racks: Vec<ProductRackTO> = serde_json::from_slice(&body).unwrap();

    // Mock service returns empty array
    assert_eq!(product_racks.len(), 0);
}

#[tokio::test]
async fn test_get_products_in_rack() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/product-racks/rack/a1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let product_racks: Vec<ProductRackTO> = serde_json::from_slice(&body).unwrap();

    // Mock service returns empty array
    assert_eq!(product_racks.len(), 0);
}

#[tokio::test]
async fn test_get_all_product_rack_relationships() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/product-racks/all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let product_racks: Vec<ProductRackTO> = serde_json::from_slice(&body).unwrap();

    // Mock service returns empty array
    assert_eq!(product_racks.len(), 0);
}

// Container Tests

#[tokio::test]
async fn test_get_all_containers() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/containers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let containers: Vec<ContainerTO> = serde_json::from_slice(&body).unwrap();

    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].name, "Test Container");
    assert_eq!(containers[0].weight_grams, 500);
    assert_eq!(containers[0].description, "A test container for storage");
}

#[tokio::test]
async fn test_get_container_by_id() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/containers/c1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let container: ContainerTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(container.name, "Test Container");
    assert_eq!(container.weight_grams, 500);
    assert_eq!(container.description, "A test container for storage");
}

#[tokio::test]
async fn test_get_container_not_found() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/containers/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_container() {
    let app = create_test_app();
    let new_container = json!({
        "name": "Storage Container B",
        "weight_grams": 750,
        "description": "Secondary storage container"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/containers")
                .header("content-type", "application/json")
                .body(Body::from(new_container.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let container: ContainerTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(container.name, "Storage Container B");
    assert_eq!(container.weight_grams, 750);
    assert_eq!(container.description, "Secondary storage container");
    assert!(container.id.is_some());
}

#[tokio::test]
async fn test_update_container() {
    let app = create_test_app();
    let updated_container = json!({
        "name": "Updated Container A",
        "weight_grams": 600,
        "description": "Updated description for container A"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/containers/c1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .header("content-type", "application/json")
                .body(Body::from(updated_container.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let container: ContainerTO = serde_json::from_slice(&body).unwrap();

    assert_eq!(container.name, "Updated Container A");
    assert_eq!(container.weight_grams, 600);
    assert_eq!(container.description, "Updated description for container A");
}

#[tokio::test]
async fn test_update_container_not_found() {
    let app = create_test_app();
    let updated_container = json!({
        "name": "NonExistent",
        "weight_grams": 100,
        "description": "This container does not exist"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/containers/00000000-0000-0000-0000-000000000000")
                .header("content-type", "application/json")
                .body(Body::from(updated_container.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_container() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/containers/c1b2c3d4-e5f6-7890-abcd-ef1234567890")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_container_not_found() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/containers/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
