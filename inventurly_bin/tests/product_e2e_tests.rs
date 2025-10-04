// Skip all tests when OIDC feature is enabled since they require a real OIDC server
#[cfg(not(feature = "oidc"))]
mod tests {
    use inventurly_bin::RestStateImpl;
    use inventurly_rest::test_server::test_support::{start_test_server, TestServer};
    use inventurly_rest_types::{ProductTO, Price};
    use reqwest::Client;
    use serde_json::json;
    use sqlx::SqlitePool;
    use std::sync::Arc;

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
    
    async fn create_product(&self, ean: &str, name: &str, price_cents: i64) -> ProductTO {
        let response = self
            .client
            .post(self.url("/products"))
            .json(&json!({
                "ean": ean,
                "name": name,
                "short_name": &name[..name.len().min(10)],
                "sales_unit": "St",
                "requires_weighing": false,
                "price": price_cents
            }))
            .send()
            .await
            .expect("Request failed");
        
        assert_eq!(response.status(), 200, "Create product failed");
        
        response
            .json::<ProductTO>()
            .await
            .expect("Failed to parse response")
    }
    
    async fn get_product(&self, ean: &str) -> Option<ProductTO> {
        let response = self
            .client
            .get(self.url(&format!("/products/{}", ean)))
            .send()
            .await
            .expect("Request failed");
        
        match response.status().as_u16() {
            200 => Some(response.json::<ProductTO>().await.expect("Failed to parse response")),
            404 => None,
            status => panic!("Unexpected status: {}", status),
        }
    }
    
    async fn get_all_products(&self) -> Vec<ProductTO> {
        let response = self
            .client
            .get(self.url("/products"))
            .send()
            .await
            .expect("Request failed");
        
        assert_eq!(response.status(), 200, "Get all products failed");
        
        response
            .json::<Vec<ProductTO>>()
            .await
            .expect("Failed to parse response")
    }
    
    async fn update_product_with_version(&self, ean: &str, product: &ProductTO, name: &str, price_cents: i64) -> Result<ProductTO, u16> {
        let response = self
            .client
            .put(self.url(&format!("/products/{}", ean)))
            .json(&json!({
                "ean": product.ean,
                "name": name,
                "short_name": &name[..name.len().min(10)],
                "sales_unit": product.sales_unit,
                "requires_weighing": product.requires_weighing,
                "price": price_cents,
                "version": product.version
            }))
            .send()
            .await
            .expect("Request failed");
        
        let status = response.status().as_u16();
        match status {
            200 => Ok(response.json::<ProductTO>().await.expect("Failed to parse response")),
            _ => {
                let body = response.text().await.unwrap_or_else(|_| "No body".to_string());
                eprintln!("Update failed with status {}: {}", status, body);
                Err(status)
            }
        }
    }
    
    async fn delete_product(&self, ean: &str) -> u16 {
        let response = self
            .client
            .delete(self.url(&format!("/products/{}", ean)))
            .send()
            .await
            .expect("Request failed");
        
        response.status().as_u16()
    }
}

#[tokio::test]
async fn test_e2e_create_product() {
    let ctx = TestContext::new().await;
    
    let product = ctx.create_product("4260474470041", "Test Product", 599).await;
    
    assert_eq!(product.ean, "4260474470041");
    assert_eq!(product.name, "Test Product");
    assert_eq!(product.short_name, "Test Produ");
    assert_eq!(product.price, Price::from_cents(599));
    assert_eq!(product.requires_weighing, false);
    assert!(product.id.is_some());
}

#[tokio::test]
async fn test_e2e_get_product() {
    let ctx = TestContext::new().await;
    
    // Create a product
    let created = ctx.create_product("4260474470041", "Test Product", 599).await;
    
    // Get the product
    let fetched = ctx.get_product("4260474470041").await;
    
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.ean, "4260474470041");
    assert_eq!(fetched.name, "Test Product");
    assert_eq!(fetched.price, Price::from_cents(599));
}

#[tokio::test]
async fn test_e2e_get_nonexistent_product() {
    let ctx = TestContext::new().await;
    
    let product = ctx.get_product("9999999999999").await;
    
    assert!(product.is_none());
}

#[tokio::test]
async fn test_e2e_get_all_products() {
    let ctx = TestContext::new().await;
    
    // Initially empty
    let products = ctx.get_all_products().await;
    assert_eq!(products.len(), 0);
    
    // Create some products
    ctx.create_product("111", "Product A", 100).await;
    ctx.create_product("222", "Product B", 200).await;
    ctx.create_product("333", "Product C", 300).await;
    
    // Get all products
    let products = ctx.get_all_products().await;
    assert_eq!(products.len(), 3);
    
    // Check EANs are present
    let eans: Vec<String> = products.iter().map(|p| p.ean.clone()).collect();
    assert!(eans.contains(&"111".to_string()));
    assert!(eans.contains(&"222".to_string()));
    assert!(eans.contains(&"333".to_string()));
}

#[tokio::test]
async fn test_e2e_update_product() {
    let ctx = TestContext::new().await;
    
    // Create a product
    let created = ctx.create_product("4260474470041", "Original Name", 599).await;
    
    // Update the product
    let updated = ctx.update_product_with_version("4260474470041", &created, "Updated Name", 799).await;
    
    assert!(updated.is_ok());
    let updated = updated.unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.price, Price::from_cents(799));
    
    // Verify the update persisted
    let fetched = ctx.get_product("4260474470041").await.expect("Product should exist");
    assert_eq!(fetched.name, "Updated Name");
    assert_eq!(fetched.price, Price::from_cents(799));
}

#[tokio::test]
async fn test_e2e_delete_product() {
    let ctx = TestContext::new().await;
    
    // Create a product
    let created = ctx.create_product("4260474470041", "To Delete", 599).await;
    let _id = created.id.expect("Product should have ID");
    
    // Delete the product
    let status = ctx.delete_product("4260474470041").await;
    assert_eq!(status, 204, "Delete should return 204 No Content");
    
    // Give a moment for the delete to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    // Verify product is deleted (should not be found)
    let product = ctx.get_product("4260474470041").await;
    assert!(product.is_none(), "Deleted product should not be found");
    
    // Verify product is not in the list
    let all = ctx.get_all_products().await;
    assert!(!all.iter().any(|p| p.ean == "4260474470041"));
}

#[tokio::test]
async fn test_e2e_delete_nonexistent_product() {
    let ctx = TestContext::new().await;
    
    let status = ctx.delete_product("9999999999999").await;
    
    assert_eq!(status, 404);
}

#[tokio::test]
async fn test_e2e_duplicate_ean() {
    let ctx = TestContext::new().await;
    
    // Create first product
    ctx.create_product("4260474470041", "First Product", 599).await;
    
    // Try to create second product with same EAN
    let response = ctx
        .client
        .post(ctx.url("/products"))
        .json(&json!({
            "ean": "4260474470041",
            "name": "Second Product",
            "short_name": "Second",
            "sales_unit": "St",
            "requires_weighing": false,
            "price": 799
        }))
        .send()
        .await
        .expect("Request failed");
    
    assert_eq!(response.status(), 400, "Should fail with 400 for duplicate EAN");
    let body = response.text().await.unwrap();
    assert!(body.contains("EAN already exists"));
}

} // End of #[cfg(not(feature = "oidc"))] mod tests