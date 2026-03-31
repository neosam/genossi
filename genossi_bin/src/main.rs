use genossi_bin::RestStateImpl;
use sqlx::SqlitePool;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter("genossi=debug,tower_http=debug")
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:genossi.db".to_string());

    tracing::info!("Connecting to database: {}", database_url);

    let pool = Arc::new(
        SqlitePool::connect(&database_url)
            .await
            .expect("Failed to connect to database"),
    );

    sqlx::migrate!("../migrations/sqlite")
        .run(&*pool)
        .await
        .expect("Failed to run migrations");

    // Create RestStateImpl with all services
    let rest_state = RestStateImpl::new(pool);

    // Start server using the rest crate's start_server function
    genossi_rest::start_server(rest_state).await;
}
