use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use inventurly_rest::RestStateDef;
use inventurly_rest_types::PersonTO;
use inventurly_service::permission::{Authentication, MockContext};
use inventurly_service::person::{Person, PersonService};
use inventurly_service::product::{Product, ProductService};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone)]
struct TestRestState {
    person_service: Arc<MockPersonService>,
    product_service: Arc<MockProductService>,
}

impl RestStateDef for TestRestState {
    type PersonService = MockPersonService;
    type ProductService = MockProductService;

    fn person_service(&self) -> Arc<Self::PersonService> {
        self.person_service.clone()
    }
    
    fn product_service(&self) -> Arc<Self::ProductService> {
        self.product_service.clone()
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

fn create_test_app() -> axum::Router {
    let rest_state = TestRestState {
        person_service: Arc::new(MockPersonService),
        product_service: Arc::new(MockProductService),
    };

    axum::Router::new()
        .nest("/persons", inventurly_rest::person::generate_route())
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