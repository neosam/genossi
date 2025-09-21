use inventurly_bin::RestStateImpl;
use inventurly_rest::test_server::test_support::{start_test_server, TestServer};
use inventurly_rest_types::PersonTO;
use reqwest::Client;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

struct TestContext {
    client: Client,
    server: TestServer,
}

impl TestContext {
    async fn new() -> Self {
        // Create in-memory database
        let pool = Arc::new(
            SqlitePool::connect("sqlite::memory:")
                .await
                .expect("Could not connect to in-memory database"),
        );
        
        // Run migrations
        sqlx::migrate!("../migrations/sqlite")
            .run(pool.as_ref())
            .await
            .expect("Failed to run migrations");
        
        // Create REST state
        let rest_state = RestStateImpl::new(pool);
        
        // Start test server
        let server = start_test_server(rest_state).await;
        
        // Create HTTP client
        let client = Client::new();
        
        Self { client, server }
    }
    
    fn url(&self, path: &str) -> String {
        self.server.url(path)
    }
    
    async fn create_person(&self, name: &str, age: i32) -> PersonTO {
        let response = self
            .client
            .post(self.url("/persons"))
            .json(&json!({
                "name": name,
                "age": age
            }))
            .send()
            .await
            .expect("Request failed");
        
        assert_eq!(response.status(), 200, "Create person failed");
        
        response
            .json::<PersonTO>()
            .await
            .expect("Failed to parse response")
    }
    
    async fn get_person(&self, id: Uuid) -> Option<PersonTO> {
        let response = self
            .client
            .get(self.url(&format!("/persons/{}", id)))
            .send()
            .await
            .expect("Request failed");
        
        match response.status().as_u16() {
            200 => Some(response.json::<PersonTO>().await.expect("Failed to parse response")),
            404 => None,
            status => panic!("Unexpected status: {}", status),
        }
    }
    
    async fn get_all_persons(&self) -> Vec<PersonTO> {
        let response = self
            .client
            .get(self.url("/persons"))
            .send()
            .await
            .expect("Request failed");
        
        assert_eq!(response.status(), 200, "Get all persons failed");
        
        response
            .json::<Vec<PersonTO>>()
            .await
            .expect("Failed to parse response")
    }
    
    async fn update_person_with_version(&self, person: &PersonTO, name: &str, age: i32) -> Result<PersonTO, u16> {
        let response = self
            .client
            .put(self.url(&format!("/persons/{}", person.id.unwrap())))
            .json(&json!({
                "name": name,
                "age": age,
                "version": person.version
            }))
            .send()
            .await
            .expect("Request failed");
        
        let status = response.status().as_u16();
        match status {
            200 => Ok(response.json::<PersonTO>().await.expect("Failed to parse response")),
            _ => {
                let body = response.text().await.unwrap_or_else(|_| "No body".to_string());
                eprintln!("Update failed with status {}: {}", status, body);
                Err(status)
            }
        }
    }
    
    async fn delete_person(&self, id: Uuid) -> u16 {
        let response = self
            .client
            .delete(self.url(&format!("/persons/{}", id)))
            .send()
            .await
            .expect("Request failed");
        
        response.status().as_u16()
    }
}

#[tokio::test]
async fn test_e2e_create_person() {
    let ctx = TestContext::new().await;
    
    let person = ctx.create_person("John Doe", 30).await;
    
    assert_eq!(person.name, "John Doe");
    assert_eq!(person.age, 30);
    assert!(person.id.is_some());
}

#[tokio::test]
async fn test_e2e_get_person() {
    let ctx = TestContext::new().await;
    
    // Create a person
    let created = ctx.create_person("Jane Smith", 25).await;
    let id = created.id.expect("Person should have ID");
    
    // Get the person
    let fetched = ctx.get_person(id).await;
    
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, Some(id));
    assert_eq!(fetched.name, "Jane Smith");
    assert_eq!(fetched.age, 25);
}

#[tokio::test]
async fn test_e2e_get_nonexistent_person() {
    let ctx = TestContext::new().await;
    
    let random_id = Uuid::new_v4();
    let person = ctx.get_person(random_id).await;
    
    assert!(person.is_none());
}

#[tokio::test]
async fn test_e2e_get_all_persons() {
    let ctx = TestContext::new().await;
    
    // Initially empty
    let persons = ctx.get_all_persons().await;
    assert_eq!(persons.len(), 0);
    
    // Create some persons
    ctx.create_person("Alice", 30).await;
    ctx.create_person("Bob", 35).await;
    ctx.create_person("Charlie", 40).await;
    
    // Get all persons
    let persons = ctx.get_all_persons().await;
    assert_eq!(persons.len(), 3);
    
    // Check names are present
    let names: Vec<String> = persons.iter().map(|p| p.name.clone()).collect();
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Bob".to_string()));
    assert!(names.contains(&"Charlie".to_string()));
}

#[tokio::test]
async fn test_e2e_update_person() {
    let ctx = TestContext::new().await;
    
    // Create a person
    let created = ctx.create_person("Original Name", 20).await;
    let id = created.id.expect("Person should have ID");
    
    // Update the person (using the version from created person)
    let updated = ctx.update_person_with_version(&created, "Updated Name", 21).await;
    
    assert!(updated.is_ok());
    let updated = updated.unwrap();
    assert_eq!(updated.id, Some(id));
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.age, 21);
    
    // Verify the update persisted
    let fetched = ctx.get_person(id).await.expect("Person should exist");
    assert_eq!(fetched.name, "Updated Name");
    assert_eq!(fetched.age, 21);
}

#[tokio::test]
async fn test_e2e_update_nonexistent_person() {
    let ctx = TestContext::new().await;
    
    let fake_person = PersonTO {
        id: Some(Uuid::new_v4()),
        name: "Fake".to_string(),
        age: 99,
        created: None,
        deleted: None,
        version: Some(Uuid::new_v4()),
    };
    let result = ctx.update_person_with_version(&fake_person, "Should Fail", 99).await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), 404);
}

#[tokio::test]
async fn test_e2e_delete_person() {
    let ctx = TestContext::new().await;
    
    // Create a person
    let created = ctx.create_person("To Delete", 50).await;
    let id = created.id.expect("Person should have ID");
    
    // Delete the person
    let status = ctx.delete_person(id).await;
    assert_eq!(status, 204, "Delete should return 204 No Content");
    
    // Give a moment for the delete to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    // Verify person is deleted (should not be found)
    let person = ctx.get_person(id).await;
    if person.is_some() {
        eprintln!("Person still found after delete: {:?}", person);
    }
    assert!(person.is_none(), "Deleted person should not be found");
    
    // Verify person is not in the list
    let all = ctx.get_all_persons().await;
    assert!(!all.iter().any(|p| p.id == Some(id)));
}

#[tokio::test]
async fn test_e2e_delete_nonexistent_person() {
    let ctx = TestContext::new().await;
    
    let random_id = Uuid::new_v4();
    let status = ctx.delete_person(random_id).await;
    
    assert_eq!(status, 404);
}

#[tokio::test]
async fn test_e2e_create_multiple_and_verify_isolation() {
    // Test that each test gets its own database
    let ctx = TestContext::new().await;
    
    // Should start with empty database
    let persons = ctx.get_all_persons().await;
    assert_eq!(persons.len(), 0, "Each test should start with empty database");
    
    // Create persons in this test
    ctx.create_person("Test1", 1).await;
    ctx.create_person("Test2", 2).await;
    
    let persons = ctx.get_all_persons().await;
    assert_eq!(persons.len(), 2);
}

#[tokio::test]
async fn test_e2e_concurrent_operations() {
    let ctx = TestContext::new().await;
    
    // Create multiple persons concurrently
    let futures = vec![
        ctx.create_person("Concurrent1", 31),
        ctx.create_person("Concurrent2", 32),
        ctx.create_person("Concurrent3", 33),
    ];
    
    let results = futures::future::join_all(futures).await;
    
    assert_eq!(results.len(), 3);
    for person in &results {
        assert!(person.id.is_some());
    }
    
    // Verify all were created
    let all = ctx.get_all_persons().await;
    assert_eq!(all.len(), 3);
}