// Skip all tests when OIDC feature is enabled since they require a real OIDC server
#[cfg(not(feature = "oidc"))]
mod tests {
    use inventurly_bin::RestStateImpl;
    use inventurly_rest::test_server::test_support::{start_test_server, TestServer};
    use inventurly_rest_types::ProductTO;
    use inventurly_service::csv_import::CsvImportResult;
    use reqwest::{multipart, Client, StatusCode};
    use sqlx::SqlitePool;
    use std::{collections::HashMap, sync::Arc};

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

    async fn upload_csv(&self, csv_content: &str) -> reqwest::Result<reqwest::Response> {
        let form = multipart::Form::new()
            .text("file", csv_content.to_string());

        self.client
            .post(self.url("/csv-import/products"))
            .multipart(form)
            .send()
            .await
    }

    async fn get_all_products(&self) -> Vec<ProductTO> {
        let response = self
            .client
            .get(self.url("/products"))
            .send()
            .await
            .expect("Failed to get products");

        response
            .json::<Vec<ProductTO>>()
            .await
            .expect("Failed to parse products JSON")
    }
}

#[tokio::test]
async fn test_csv_upload_create_new_products() {
    let ctx = TestContext::new().await;
    
    // Sample CSV content with German format
    let csv_content = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst\n4260474470041,Macadamia süss salzig,Macadamia süss,130g,0,\"5,39\"\n4260474470058,Cashew geröstet,Cashew geröstet,200g,0,\"7,89\"\n4260474470065,Mandeln geschält,Mandeln,250g,9,\"4,99\"";

    // Upload CSV and verify response
    let response = ctx.upload_csv(csv_content).await.expect("Failed to upload CSV");
    let status = response.status();
    if status != StatusCode::OK {
        let error_body = response.text().await.unwrap();
        eprintln!("Error response ({}): {}", status, error_body);
        panic!("Expected 200 OK, got {}", status);
    }
    assert_eq!(status, StatusCode::OK);
    
    let result: CsvImportResult = response.json().await.expect("Failed to parse response JSON");

    // Verify import results
    assert_eq!(result.total_rows, 3);
    assert_eq!(result.created, 3);
    assert_eq!(result.updated, 0);
    assert_eq!(result.errors.len(), 0);

    // Verify products were actually created in database
    let products = ctx.get_all_products().await;
    assert_eq!(products.len(), 3);
    
    // Verify specific product details
    let macadamia = products.iter().find(|p| p.ean == "4260474470041").unwrap();
    assert_eq!(macadamia.name, "Macadamia süss salzig");
    assert_eq!(macadamia.short_name, "Macadamia süss");
    assert_eq!(macadamia.sales_unit, "130g");
    assert_eq!(macadamia.requires_weighing, false);
    assert_eq!(macadamia.price.to_cents(), 539); // 5,39 euros = 539 cents

    let mandeln = products.iter().find(|p| p.ean == "4260474470065").unwrap();
    assert_eq!(mandeln.requires_weighing, true); // WiegeArtikel = 9
    assert_eq!(mandeln.price.to_cents(), 499); // 4,99 euros = 499 cents
}

#[tokio::test]
async fn test_csv_upload_update_existing_products() {
    let ctx = TestContext::new().await;
    
    // First, create a product
    let initial_csv = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
4260474470041,Macadamia original,Macadamia orig,130g,0,\"5,39\"";

    let response1 = ctx.upload_csv(initial_csv).await.expect("Failed to upload initial CSV");
    assert_eq!(response1.status(), StatusCode::OK);
    let result1: CsvImportResult = response1.json().await.unwrap();
    assert_eq!(result1.created, 1);
    assert_eq!(result1.updated, 0);

    // Now update the same product with different data
    let updated_csv = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
4260474470041,Macadamia süss salzig UPDATED,Macadamia UPDATED,150g,9,\"6,49\"";

    let response2 = ctx.upload_csv(updated_csv).await.expect("Failed to upload update CSV");
    assert_eq!(response2.status(), StatusCode::OK);
    let result2: CsvImportResult = response2.json().await.unwrap();
    assert_eq!(result2.total_rows, 1);
    assert_eq!(result2.created, 0);
    assert_eq!(result2.updated, 1);
    assert_eq!(result2.errors.len(), 0);

    // Verify the product was updated
    let products = ctx.get_all_products().await;
    assert_eq!(products.len(), 1); // Should still be only 1 product

    let updated_product = &products[0];
    assert_eq!(updated_product.ean, "4260474470041");
    assert_eq!(updated_product.name, "Macadamia süss salzig UPDATED");
    assert_eq!(updated_product.short_name, "Macadamia UPDATED");
    assert_eq!(updated_product.sales_unit, "150g");
    assert_eq!(updated_product.requires_weighing, true); // Changed from 0 to 9
    assert_eq!(updated_product.price.to_cents(), 649); // 6,49 euros = 649 cents
}

#[tokio::test]
async fn test_csv_upload_mixed_create_and_update() {
    let ctx = TestContext::new().await;
    
    // First, create one product
    let initial_csv = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
4260474470041,Macadamia original,Macadamia orig,130g,0,\"5,39\"";

    ctx.upload_csv(initial_csv).await.expect("Failed to upload initial CSV");

    // Now upload CSV with one update and one new product
    let mixed_csv = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
4260474470041,Macadamia UPDATED,Macadamia UPD,140g,0,\"5,99\"
4260474470058,Cashew NEW,Cashew NEW,200g,9,\"7,89\"";

    let response = ctx.upload_csv(mixed_csv).await.expect("Failed to upload mixed CSV");
    assert_eq!(response.status(), StatusCode::OK);
    let result: CsvImportResult = response.json().await.unwrap();
    
    assert_eq!(result.total_rows, 2);
    assert_eq!(result.created, 1);  // Cashew is new
    assert_eq!(result.updated, 1);  // Macadamia is updated
    assert_eq!(result.errors.len(), 0);

    // Verify both products exist with correct data
    let products = ctx.get_all_products().await;
    assert_eq!(products.len(), 2);

    let macadamia = products.iter().find(|p| p.ean == "4260474470041").unwrap();
    assert_eq!(macadamia.name, "Macadamia UPDATED");
    assert_eq!(macadamia.price.to_cents(), 599);

    let cashew = products.iter().find(|p| p.ean == "4260474470058").unwrap();
    assert_eq!(cashew.name, "Cashew NEW");
    assert_eq!(cashew.requires_weighing, true);
    assert_eq!(cashew.price.to_cents(), 789);
}

#[tokio::test]
async fn test_csv_upload_invalid_csv_format() {
    let ctx = TestContext::new().await;
    
    // CSV with completely invalid price causing CSV parsing to fail
    let invalid_csv = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst\n4260474470041,Macadamia,Macadamia,130g,0,\"invalid_price\"\n4260474470058,Cashew,Cashew,200g,2,\"7,89\"";

    let response = ctx.upload_csv(invalid_csv).await.expect("Failed to upload CSV");
    // Should return 400 for completely invalid price format - this is correct behavior
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_csv_upload_business_validation_errors() {
    let ctx = TestContext::new().await;
    
    // CSV with valid CSV format but invalid business logic values
    let invalid_csv = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst\n4260474470041,,Macadamia,130g,0,\"5,39\"\n4260474470058,Cashew,Cashew,200g,5,\"7,89\"";  // Empty name and invalid WiegeArtikel

    let response = ctx.upload_csv(invalid_csv).await.expect("Failed to upload CSV");
    // Should return 400 for business validation errors - this is correct behavior
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_csv_upload_missing_columns() {
    let ctx = TestContext::new().await;
    
    // CSV missing required columns
    let invalid_csv = "EAN,Bezeichnung
4260474470041,Macadamia";

    let response = ctx.upload_csv(invalid_csv).await.expect("Failed to upload CSV");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_csv_upload_empty_file() {
    let ctx = TestContext::new().await;
    
    let response = ctx.upload_csv("").await.expect("Failed to upload CSV");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_csv_upload_german_decimal_formats() {
    let ctx = TestContext::new().await;
    
    // Test various German decimal formats
    let csv_content = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
4260474470041,Product1,P1,100g,0,\"5,39\"
4260474470042,Product2,P2,100g,0,\"10,00\"
4260474470043,Product3,P3,100g,0,\"0,99\"
4260474470044,Product4,P4,100g,0,\"15,50\"";

    let response = ctx.upload_csv(csv_content).await.expect("Failed to upload CSV");
    assert_eq!(response.status(), StatusCode::OK);
    
    let result: CsvImportResult = response.json().await.unwrap();
    assert_eq!(result.created, 4);
    assert_eq!(result.errors.len(), 0);

    // Verify prices were converted correctly
    let products = ctx.get_all_products().await;
    
    let prices: HashMap<&str, i64> = products.iter()
        .map(|p| (p.ean.as_str(), p.price.to_cents()))
        .collect();

    assert_eq!(prices["4260474470041"], 539);  // 5,39 -> 539 cents
    assert_eq!(prices["4260474470042"], 1000); // 10,00 -> 1000 cents
    assert_eq!(prices["4260474470043"], 99);   // 0,99 -> 99 cents
    assert_eq!(prices["4260474470044"], 1550); // 15,50 -> 1550 cents
}

#[tokio::test]
async fn test_csv_upload_skip_empty_ean_rows() {
    let ctx = TestContext::new().await;
    
    // CSV with some empty EAN rows that should be skipped
    let csv_content = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
4260474470041,Product1,P1,100g,0,\"5,39\"
,Empty EAN,Empty,100g,0,\"1,00\"
4260474470042,Product2,P2,100g,0,\"7,89\"
  ,Whitespace EAN,WS,100g,0,\"2,00\"";

    let response = ctx.upload_csv(csv_content).await.expect("Failed to upload CSV");
    assert_eq!(response.status(), StatusCode::OK);
    
    let result: CsvImportResult = response.json().await.unwrap();
    assert_eq!(result.total_rows, 2); // Only 2 valid rows processed
    assert_eq!(result.created, 2);
    assert_eq!(result.errors.len(), 0);

    // Verify only valid products were created
    let products = ctx.get_all_products().await;
    assert_eq!(products.len(), 2);
    
    let eans: Vec<&str> = products.iter().map(|p| p.ean.as_str()).collect();
    assert!(eans.contains(&"4260474470041"));
    assert!(eans.contains(&"4260474470042"));
}

} // End of #[cfg(not(feature = "oidc"))] mod tests