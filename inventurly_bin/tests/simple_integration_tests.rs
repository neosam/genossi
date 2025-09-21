use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use inventurly_bin::RestStateImpl;
use inventurly_rest::RestStateDef;
use inventurly_rest_types::PersonTO;
use inventurly_service::permission::Authentication;
use inventurly_service::person::PersonService;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

fn create_test_app(rest_state: RestStateImpl) -> axum::Router {
    axum::Router::new()
        .nest("/persons", inventurly_rest::person::generate_route())
        .with_state(rest_state)
        .layer(axum::middleware::from_fn(|mut req: Request<Body>, next: axum::middleware::Next| async move {
            req.extensions_mut().insert(inventurly_rest::Context::default());
            next.run(req).await
        }))
}

async fn setup_test() -> RestStateImpl {
    let pool = Arc::new(
        SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Could not connect to database"),
    );
    
    sqlx::migrate!("../migrations/sqlite")
        .run(pool.as_ref())
        .await
        .expect("Failed to run migrations");

    RestStateImpl::new(pool)
}

#[tokio::test]
async fn test_simple_create_person() {
    let rest_state = setup_test().await;
    let app = create_test_app(rest_state);

    let new_person = json!({
        "name": "Test Person",
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
    
    assert_eq!(person.name, "Test Person");
    assert_eq!(person.age, 25);
    assert!(person.id.is_some());
}

#[tokio::test]
async fn test_simple_get_all_empty() {
    let rest_state = setup_test().await;
    let app = create_test_app(rest_state);

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
    
    assert_eq!(persons.len(), 0);
}

#[tokio::test]
async fn test_create_and_get_person_integration() {
    let rest_state = setup_test().await;

    // First create a person
    let new_person = json!({
        "name": "John Doe",
        "age": 30
    });

    let app = create_test_app(rest_state.clone());
    let create_response = app
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

    assert_eq!(create_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created_person: PersonTO = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(created_person.name, "John Doe");
    assert_eq!(created_person.age, 30);
    assert!(created_person.id.is_some());
    
    let person_id = created_person.id.unwrap();

    // Verify the person was stored in database via service layer
    let person_from_db = rest_state
        .person_service()
        .get(person_id, Authentication::Context(inventurly_service::permission::MockContext), None)
        .await
        .unwrap();
    
    assert_eq!(person_from_db.id, person_id);
    assert_eq!(person_from_db.name.as_ref(), "John Doe");
    assert_eq!(person_from_db.age, 30);
}

#[tokio::test]
async fn test_get_nonexistent_person() {
    let rest_state = setup_test().await;
    let app = create_test_app(rest_state);

    let nonexistent_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/persons/{}", nonexistent_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]  
async fn test_create_multiple_persons_and_get_all() {
    let rest_state = setup_test().await;

    // Create first person
    let person1 = json!({
        "name": "Alice Smith",
        "age": 25
    });

    let app = create_test_app(rest_state.clone());
    let create_response1 = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/persons")
                .header("Content-Type", "application/json")
                .body(Body::from(person1.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response1.status(), StatusCode::OK);

    // Create second person using service layer directly
    use inventurly_service::person::Person;
    
    let person2 = Person {
        id: Uuid::nil(),
        name: Arc::from("Bob Johnson"),
        age: 35,
        created: time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
            time::Time::from_hms(0, 0, 0).unwrap(),
        ),
        deleted: None,
        version: Uuid::nil(),
    };
    
    let _created_person2 = rest_state
        .person_service()
        .create(&person2, Authentication::Context(inventurly_service::permission::MockContext), None)
        .await
        .unwrap();

    // Now get all persons
    let app = create_test_app(rest_state);
    let get_all_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/persons")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_all_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(get_all_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let persons: Vec<PersonTO> = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(persons.len(), 2);
    
    // Find persons by name (order may vary)
    let alice = persons.iter().find(|p| p.name == "Alice Smith");
    let bob = persons.iter().find(|p| p.name == "Bob Johnson");
    
    assert!(alice.is_some());
    assert!(bob.is_some());
    assert_eq!(alice.unwrap().age, 25);
    assert_eq!(bob.unwrap().age, 35);
}

#[tokio::test]
async fn test_update_nonexistent_person() {
    let rest_state = setup_test().await;
    let app = create_test_app(rest_state);

    let nonexistent_id = Uuid::new_v4();
    let update_data = json!({
        "name": "Does Not Exist",
        "age": 99
    });

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("/persons/{}", nonexistent_id))
                .header("Content-Type", "application/json")
                .body(Body::from(update_data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_person() {
    let rest_state = setup_test().await;
    let app = create_test_app(rest_state);

    let nonexistent_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("/persons/{}", nonexistent_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}