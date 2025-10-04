use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use inventurly_rest::RestStateDef;
use inventurly_rest_types::{PersonTO, RackTO};
use inventurly_service::permission::{Authentication, MockContext};
use inventurly_service::person::{Person, PersonService};
use inventurly_service::product::{Product, ProductService};
use inventurly_service::rack::{Rack, RackService};
use inventurly_service::csv_import::{CsvImportService, CsvImportResult, CsvProductRow, ImportAction};
use inventurly_service::duplicate_detection::{DuplicateDetectionService, DuplicateDetectionConfig, DuplicateDetectionResult, DuplicateMatch};
use inventurly_service::permission::PermissionService;
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
    csv_import_service: Arc<MockCsvImportService>,
    duplicate_detection_service: Arc<MockDuplicateDetectionService>,
    permission_service: Arc<MockPermissionService>,
    session_service: Arc<MockSessionService>,
}

impl RestStateDef for TestRestState {
    type PersonService = MockPersonService;
    type ProductService = MockProductService;
    type RackService = MockRackService;
    type CsvImportService = MockCsvImportService;
    type DuplicateDetectionService = MockDuplicateDetectionService;
    type PermissionService = MockPermissionService;
    type SessionService = MockSessionService;

    fn person_service(&self) -> Arc<Self::PersonService> {
        self.person_service.clone()
    }
    
    fn product_service(&self) -> Arc<Self::ProductService> {
        self.product_service.clone()
    }
    
    fn rack_service(&self) -> Arc<Self::RackService> {
        self.rack_service.clone()
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

#[derive(Clone)]
struct MockPersonService;

#[async_trait::async_trait]
impl PersonService for MockPersonService {
    type Context = MockContext;
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
    type Context = MockContext;
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
        Err(inventurly_service::ServiceError::InternalError(Arc::from("Not implemented")))
    }

    async fn update(
        &self,
        _product: &Product,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<Product, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::InternalError(Arc::from("Not implemented")))
    }

    async fn delete(
        &self,
        id: Uuid,
        _auth: Authentication<Self::Context>,
        _transaction: Option<Self::Transaction>,
    ) -> Result<(), inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::EntityNotFound(id))
    }
}

#[derive(Clone)]
struct MockRackService;

#[async_trait::async_trait]
impl RackService for MockRackService {
    type Context = MockContext;
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
struct MockCsvImportService;

#[async_trait::async_trait]
impl CsvImportService for MockCsvImportService {
    type Context = MockContext;
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
    type Context = MockContext;
    type Transaction = inventurly_dao::MockTransaction;

    async fn find_duplicates(
        &self,
        _product: &Product,
        _config: Option<DuplicateDetectionConfig>,
        _context: Authentication<Self::Context>,
        _tx: Option<Self::Transaction>,
    ) -> Result<DuplicateDetectionResult, inventurly_service::ServiceError> {
        Err(inventurly_service::ServiceError::InternalError(Arc::from("Not implemented")))
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
    type Context = MockContext;
    
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
    ) -> Result<Arc<[inventurly_service::auth_types::UserResponseTO]>, inventurly_service::ServiceError> {
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
    ) -> Result<Arc<[inventurly_service::auth_types::RoleResponseTO]>, inventurly_service::ServiceError> {
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
    ) -> Result<Arc<[inventurly_service::auth_types::PrivilegeResponseTO]>, inventurly_service::ServiceError> {
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
    ) -> Result<Arc<[inventurly_service::auth_types::RoleResponseTO]>, inventurly_service::ServiceError> {
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
    ) -> Result<Arc<[inventurly_service::auth_types::PrivilegeResponseTO]>, inventurly_service::ServiceError> {
        Ok(Arc::new([]))
    }
    
    async fn get_user_privileges(
        &self,
        _username: String,
        _context: Authentication<Self::Context>,
    ) -> Result<Arc<[inventurly_service::auth_types::PrivilegeResponseTO]>, inventurly_service::ServiceError> {
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
    ) -> Result<Option<inventurly_service::auth_types::AuthContext>, inventurly_service::ServiceError> {
        Ok(Some(inventurly_service::auth_types::AuthContext::Mock(
            inventurly_service::auth_types::MockContext::default()
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
    ) -> Result<Option<inventurly_service::auth_types::UserSession>, inventurly_service::ServiceError> {
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
    
    async fn cleanup_expired_sessions(
        &self,
    ) -> Result<u64, inventurly_service::ServiceError> {
        Ok(0)
    }
}

fn create_test_app() -> axum::Router {
    let rest_state = TestRestState {
        person_service: Arc::new(MockPersonService),
        product_service: Arc::new(MockProductService),
        rack_service: Arc::new(MockRackService),
        csv_import_service: Arc::new(MockCsvImportService),
        duplicate_detection_service: Arc::new(MockDuplicateDetectionService),
        permission_service: Arc::new(MockPermissionService),
        session_service: Arc::new(MockSessionService),
    };

    axum::Router::new()
        .nest("/persons", inventurly_rest::person::generate_route())
        .nest("/racks", inventurly_rest::rack::generate_route())
        .with_state(rest_state)
        .layer(axum::middleware::from_fn(|mut req: Request<Body>, next: axum::middleware::Next| async move {
            req.extensions_mut().insert(inventurly_rest::Context::default());
            next.run(req).await
        }))
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
    assert_eq!(person.id, Some(Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap()));
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
    assert_eq!(person.id, Some(Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap()));
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
    assert_eq!(rack.id, Some(Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()));
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
    assert_eq!(rack.id, Some(Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()));
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