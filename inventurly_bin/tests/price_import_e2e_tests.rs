// Skip all tests when OIDC feature is enabled since they require a real OIDC server
#[cfg(not(feature = "oidc"))]
mod tests {
    use inventurly_bin::RestStateImpl;
    use inventurly_rest::test_server::test_support::{start_test_server, TestServer};
    use inventurly_rest_types::ProductTO;
    use inventurly_service::price_import::PriceImportResult;
    use reqwest::{multipart, Client, StatusCode};
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

        async fn upload_products_csv(&self, csv_content: &str) -> reqwest::Result<reqwest::Response> {
            let form = multipart::Form::new().text("file", csv_content.to_string());

            self.client
                .post(self.url("/csv-import/products"))
                .multipart(form)
                .send()
                .await
        }

        async fn upload_prices_csv(&self, csv_content: &str) -> reqwest::Result<reqwest::Response> {
            let form = multipart::Form::new().text("file", csv_content.to_string());

            self.client
                .post(self.url("/price-import/prices"))
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

        async fn create_test_products(&self) {
            // Create some test products using the CSV import
            let csv_content = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
11575,Ananas Extra Sweet,Ananas,St,0,\"1,00\"
11590,Erdbeeren 500g,Erdbeeren,500g,0,\"2,00\"
11600,Aprikosen IT,Aprikosen,kg,0,\"3,00\"";

            let response = self
                .upload_products_csv(csv_content)
                .await
                .expect("Failed to create test products");
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_price_import_update_existing_products() {
        let ctx = TestContext::new().await;

        // First, create some products
        ctx.create_test_products().await;

        // Verify initial prices
        let products = ctx.get_all_products().await;
        assert_eq!(products.len(), 3);
        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 100); // 1,00 euro

        // Now upload price update CSV
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas Extra Sweet,St,S,101,7,0,\"2,50 €\"
,11590,0,Erdbeeren 500g,500g,L,101,7,0,\"5,60 €\"
,11600,0,Aprikosen IT,kg,L,101,7,0,\"4,30 €\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        let status = response.status();
        if status != StatusCode::OK {
            let error_body = response.text().await.unwrap();
            eprintln!("Error response ({}): {}", status, error_body);
            panic!("Expected 200 OK, got {}", status);
        }

        let result: PriceImportResult = response
            .json()
            .await
            .expect("Failed to parse response JSON");

        // Verify import results
        assert_eq!(result.total_rows, 3);
        assert_eq!(result.updated, 3);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors.len(), 0);

        // Verify prices were updated
        let products = ctx.get_all_products().await;

        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 250); // 2,50 euro = 250 cents

        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 560); // 5,60 euro = 560 cents

        let aprikosen = products.iter().find(|p| p.ean == "11600").unwrap();
        assert_eq!(aprikosen.price.to_cents(), 430); // 4,30 euro = 430 cents
    }

    #[tokio::test]
    async fn test_price_import_product_not_found() {
        let ctx = TestContext::new().await;

        // Create only one product
        let csv_content = "EAN,Bezeichnung,Kurzbezeichnung,VKEinheit,WiegeArtikel,VKHerst
11575,Ananas Extra Sweet,Ananas,St,0,\"1,00\"";

        ctx.upload_products_csv(csv_content)
            .await
            .expect("Failed to create test product");

        // Try to update prices for products that don't exist
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas Extra Sweet,St,S,101,7,0,\"2,50 €\"
,99999,0,Unknown Product,St,S,101,7,0,\"5,00 €\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();

        assert_eq!(result.total_rows, 2);
        assert_eq!(result.updated, 1); // Only ananas was updated
        assert_eq!(result.skipped, 1); // Unknown product was skipped
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].ean, "99999");
        assert!(result.errors[0].message.contains("not found"));

        // Verify the existing product was still updated
        let products = ctx.get_all_products().await;
        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 250);
    }

    #[tokio::test]
    async fn test_price_import_german_format_with_euro_symbol() {
        let ctx = TestContext::new().await;
        ctx.create_test_products().await;

        // Test various German price formats with € symbol
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas,St,S,101,7,0,\"2,50 €\"
,11590,0,Erdbeeren,500g,L,101,7,0,\"10,99 €\"
,11600,0,Aprikosen,kg,L,101,7,0,\"0,99 €\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();
        assert_eq!(result.updated, 3);
        assert_eq!(result.errors.len(), 0);

        let products = ctx.get_all_products().await;

        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 250);

        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 1099);

        let aprikosen = products.iter().find(|p| p.ean == "11600").unwrap();
        assert_eq!(aprikosen.price.to_cents(), 99);
    }

    #[tokio::test]
    async fn test_price_import_german_format_without_euro_symbol() {
        let ctx = TestContext::new().await;
        ctx.create_test_products().await;

        // Test German format without € symbol
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas,St,S,101,7,0,\"2,50\"
,11590,0,Erdbeeren,500g,L,101,7,0,\"10,99\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();
        assert_eq!(result.updated, 2);

        let products = ctx.get_all_products().await;

        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 250);

        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 1099);
    }

    #[tokio::test]
    async fn test_price_import_period_decimal_format() {
        let ctx = TestContext::new().await;
        ctx.create_test_products().await;

        // Test period as decimal separator
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas,St,S,101,7,0,\"2.50\"
,11590,0,Erdbeeren,500g,L,101,7,0,\"10.99\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();
        assert_eq!(result.updated, 2);

        let products = ctx.get_all_products().await;

        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 250);

        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 1099);
    }

    #[tokio::test]
    async fn test_price_import_empty_and_zero_prices() {
        let ctx = TestContext::new().await;
        ctx.create_test_products().await;

        // Test empty and zero prices
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas,St,S,101,7,0,\"0\"
,11590,0,Erdbeeren,500g,L,101,7,0,\"\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();
        assert_eq!(result.updated, 2);

        let products = ctx.get_all_products().await;

        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 0);

        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 0);
    }

    #[tokio::test]
    async fn test_price_import_missing_columns() {
        let ctx = TestContext::new().await;

        // CSV missing EKN column
        let invalid_csv = ",EAN,,Name
,11575,0,Ananas";

        let response = ctx
            .upload_prices_csv(invalid_csv)
            .await
            .expect("Failed to upload CSV");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_price_import_empty_file() {
        let ctx = TestContext::new().await;

        let response = ctx
            .upload_prices_csv("")
            .await
            .expect("Failed to upload CSV");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_price_import_skip_empty_ean_rows() {
        let ctx = TestContext::new().await;
        ctx.create_test_products().await;

        // CSV with some empty EAN rows that should be skipped
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas,St,S,101,7,0,\"5,00 €\"
,,0,Empty EAN,St,S,101,7,0,\"1,00 €\"
,11590,0,Erdbeeren,500g,L,101,7,0,\"7,00 €\"
,  ,0,Whitespace EAN,St,S,101,7,0,\"2,00 €\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();
        assert_eq!(result.total_rows, 2); // Only 2 valid rows processed
        assert_eq!(result.updated, 2);
        assert_eq!(result.errors.len(), 0);

        let products = ctx.get_all_products().await;

        let ananas = products.iter().find(|p| p.ean == "11575").unwrap();
        assert_eq!(ananas.price.to_cents(), 500);

        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 700);
    }

    #[tokio::test]
    async fn test_price_import_invalid_price_format() {
        let ctx = TestContext::new().await;
        ctx.create_test_products().await;

        // CSV with invalid price format
        let price_csv = ",EAN,,Name,,,,,,EKN
,11575,0,Ananas,St,S,101,7,0,\"invalid_price\"
,11590,0,Erdbeeren,500g,L,101,7,0,\"5,00 €\"";

        let response = ctx
            .upload_prices_csv(price_csv)
            .await
            .expect("Failed to upload price CSV");

        assert_eq!(response.status(), StatusCode::OK);

        let result: PriceImportResult = response.json().await.unwrap();
        assert_eq!(result.total_rows, 2);
        assert_eq!(result.updated, 1); // Only erdbeeren was updated
        assert_eq!(result.skipped, 1); // Ananas had invalid price
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].ean, "11575");
        assert!(result.errors[0].message.contains("Invalid price"));

        // Verify erdbeeren was still updated
        let products = ctx.get_all_products().await;
        let erdbeeren = products.iter().find(|p| p.ean == "11590").unwrap();
        assert_eq!(erdbeeren.price.to_cents(), 500);
    }
} // End of #[cfg(not(feature = "oidc"))] mod tests
