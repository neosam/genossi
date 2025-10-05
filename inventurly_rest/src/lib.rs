pub mod auth;
pub mod auth_middleware;
pub mod csv_import;
pub mod duplicate_detection;
pub mod permission;
pub mod person;
pub mod product;
pub mod product_rack;
pub mod rack;
pub mod session;
pub mod test_server;

use async_trait::async_trait;
use axum::{body::Body, middleware, response::Response, Router};
use inventurly_service::{
    auth_types::AuthContext,
    permission::{Authentication, MockContext},
};
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "oidc")]
use axum::response::{IntoResponse, Redirect};
#[cfg(feature = "oidc")]
use axum::routing::get;

#[derive(Clone, Debug)]
pub struct Context {
    pub auth: Authentication<MockContext>,
    pub auth_context: Option<AuthContext>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            auth: Authentication::Context(MockContext),
            auth_context: Some(AuthContext::Mock(
                inventurly_service::auth_types::MockContext::default(),
            )),
        }
    }
}

pub enum RestError {
    NotFound,
    BadRequest(String),
    Unauthorized,
    InternalError(String),
}

impl From<inventurly_service::ServiceError> for RestError {
    fn from(e: inventurly_service::ServiceError) -> Self {
        match e {
            inventurly_service::ServiceError::EntityNotFound(_) => RestError::NotFound,
            inventurly_service::ServiceError::ValidationError(items) => {
                let messages: Vec<String> = items
                    .iter()
                    .map(|i| format!("{}: {}", i.field, i.message))
                    .collect();
                RestError::BadRequest(messages.join(", "))
            }
            inventurly_service::ServiceError::PermissionDenied => RestError::Unauthorized,
            _ => RestError::InternalError(format!("{:?}", e)),
        }
    }
}

pub fn error_handler(result: Result<Response, RestError>) -> Response {
    match result {
        Ok(response) => response,
        Err(RestError::NotFound) => Response::builder()
            .status(404)
            .body(Body::from("Not found"))
            .unwrap(),
        Err(RestError::BadRequest(msg)) => Response::builder()
            .status(400)
            .body(Body::from(msg))
            .unwrap(),
        Err(RestError::Unauthorized) => Response::builder()
            .status(401)
            .body(Body::from("Unauthorized"))
            .unwrap(),
        Err(RestError::InternalError(msg)) => {
            tracing::error!("Internal error: {}", msg);
            Response::builder()
                .status(500)
                .body(Body::from("Internal server error"))
                .unwrap()
        }
    }
}

#[async_trait]
pub trait RestStateDef: Clone + Send + Sync + 'static {
    type PersonService: inventurly_service::person::PersonService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type ProductService: inventurly_service::product::ProductService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type CsvImportService: inventurly_service::csv_import::CsvImportService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type DuplicateDetectionService: inventurly_service::duplicate_detection::DuplicateDetectionService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type PermissionService: inventurly_service::permission::PermissionService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type RackService: inventurly_service::rack::RackService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type ProductRackService: inventurly_service::product_rack::ProductRackService<Context = MockContext>
        + Send
        + Sync
        + 'static;
    type SessionService: inventurly_service::session::SessionService + Send + Sync + 'static;

    fn person_service(&self) -> Arc<Self::PersonService>;
    fn product_service(&self) -> Arc<Self::ProductService>;
    fn rack_service(&self) -> Arc<Self::RackService>;
    fn product_rack_service(&self) -> Arc<Self::ProductRackService>;
    fn csv_import_service(&self) -> Arc<Self::CsvImportService>;
    fn duplicate_detection_service(&self) -> Arc<Self::DuplicateDetectionService>;
    fn permission_service(&self) -> Arc<Self::PermissionService>;
    fn session_service(&self) -> Arc<Self::SessionService>;
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/auth", api = auth::ApiDoc),
        (path = "/persons", api = person::ApiDoc),
        (path = "/products", api = product::ApiDoc),
        (path = "/racks", api = rack::ApiDoc),
        (path = "/product-racks", api = product_rack::ApiDoc),
        (path = "/csv-import", api = csv_import::CsvImportApiDoc),
        (path = "/duplicate-detection", api = duplicate_detection::DuplicateDetectionApiDoc),
        (path = "/permission", api = permission::ApiDoc)
    )
)]
pub struct ApiDoc;

pub fn bind_address() -> Arc<str> {
    std::env::var("SERVER_ADDRESS")
        .unwrap_or("0.0.0.0:3000".into())
        .into()
}

#[cfg(feature = "oidc")]
pub struct OidcConfig {
    pub app_url: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[cfg(feature = "oidc")]
pub fn oidc_config() -> OidcConfig {
    let app_url = std::env::var("APP_URL").expect("APP_URL env variable");
    let issuer = std::env::var("ISSUER").expect("ISSUER env variable");
    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID env variable");
    let client_secret = std::env::var("CLIENT_SECRET").ok();
    
    // Debug logging for OIDC configuration
    tracing::info!("OIDC Configuration:");
    tracing::info!("  APP_URL: {}", app_url);
    tracing::info!("  ISSUER: {}", issuer);
    tracing::info!("  CLIENT_ID: {}", client_id);
    tracing::info!("  CLIENT_SECRET: {}", if client_secret.is_some() { "***PROVIDED***" } else { "NOT_SET" });
    
    let filtered_secret = client_secret.filter(|s| !s.is_empty());
    if filtered_secret.is_none() {
        tracing::warn!("CLIENT_SECRET is empty or not set - this may cause authentication failures");
    }
    
    OidcConfig {
        app_url,
        issuer,
        client_id,
        client_secret: filtered_secret,
    }
}

#[cfg(feature = "oidc")]
pub async fn login() -> Redirect {
    Redirect::to("/")
}

#[cfg(feature = "oidc")]
use axum_oidc::OidcRpInitiatedLogout;
#[cfg(feature = "oidc")]
use http::StatusCode;

#[cfg(feature = "oidc")]
pub async fn logout(logout_extractor: OidcRpInitiatedLogout) -> Result<Redirect, StatusCode> {
    if let Ok(logout_uri) = logout_extractor.uri() {
        Ok(Redirect::to(&format!("{}", logout_uri)))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

// OIDC takes priority over mock_auth when both are enabled
#[cfg(feature = "oidc")]
async fn context_extractor<RestState: RestStateDef>(
    rest_state: axum::extract::State<RestState>,
    request: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    session::context_extractor(rest_state, request, next).await
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
async fn context_extractor<RestState: RestStateDef>(
    rest_state: axum::extract::State<RestState>,
    request: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    session::context_extractor(rest_state, request, next).await
}


pub async fn create_app<RestState: RestStateDef>(rest_state: RestState) -> Router {
    let mut api_doc = ApiDoc::openapi();
    let base = std::env::var("BASE_PATH").unwrap_or("http://localhost:3000/".into());
    api_doc.servers = Some(vec![utoipa::openapi::ServerBuilder::new()
        .url(base)
        .description(Some("Inventurly backend"))
        .build()]);

    let swagger_router = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_doc);

    let mut app = Router::new()
        .merge(swagger_router);

    #[cfg(feature = "oidc")]
    {
        app = app.route("/authenticate", get(login));
    }

    let app = app
        .nest("/auth", auth::generate_route())
        .nest("/persons", person::generate_route())
        .nest("/products", product::generate_route())
        .nest("/racks", rack::generate_route())
        .nest("/product-racks", product_rack::generate_route())
        .nest("/csv-import", csv_import::generate_route())
        .nest(
            "/duplicate-detection",
            duplicate_detection::generate_route(),
        )
        .nest("/permission", permission::generate_route())
        .with_state(rest_state.clone())
        .layer(middleware::from_fn_with_state(
            rest_state.clone(),
            session::forbid_unauthenticated::<RestState>,
        ))
        .layer(middleware::from_fn_with_state(
            rest_state.clone(),
            context_extractor::<RestState>,
        ))
        .layer(CorsLayer::permissive());

    #[cfg(feature = "oidc")]
    let app = {
        use axum::error_handling::HandleErrorLayer;
        use axum_oidc::error::MiddlewareError;
        use axum_oidc::{EmptyAdditionalClaims, OidcAuthLayer, OidcLoginLayer};
        use http::Uri;
        use time::Duration;
        use tower::ServiceBuilder;
        use tower_sessions::cookie::SameSite;
        use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

        let oidc_config = oidc_config();
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(true)
            .with_same_site(SameSite::Strict)
            .with_expiry(Expiry::OnInactivity(Duration::minutes(50)));

        let oidc_login_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
                tracing::error!("OIDC Login error: {:?}", e);
                e.into_response()
            }))
            .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());

        tracing::info!("Attempting OIDC client discovery...");
        let oidc_auth_layer_result = OidcAuthLayer::<EmptyAdditionalClaims>::discover_client(
            Uri::from_maybe_shared(oidc_config.app_url.clone()).expect("valid APP_URL"),
            oidc_config.issuer.clone(),
            oidc_config.client_id.clone(),
            oidc_config.client_secret.clone(),
            vec![],
        )
        .await;

        let oidc_auth_layer = match oidc_auth_layer_result {
            Ok(layer) => {
                tracing::info!("OIDC client discovery successful");
                layer
            },
            Err(e) => {
                tracing::error!("OIDC client discovery failed: {:?}", e);
                tracing::error!("Check your OIDC configuration:");
                tracing::error!("  - Issuer URL is accessible: {}", oidc_config.issuer);
                tracing::error!("  - Client ID is correct: {}", oidc_config.client_id);
                tracing::error!("  - App URL is correct: {}", oidc_config.app_url);
                panic!("Failed to discover OIDC client: {:?}", e);
            }
        };

        let oidc_auth_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
                tracing::error!("OIDC Auth error: {:?}", e);
                e.into_response()
            }))
            .layer(oidc_auth_layer);

        // Add logout route with OIDC support
        app
            .route("/logout", get(logout))
            .layer(middleware::from_fn_with_state(
                rest_state.clone(),
                session::register_session::<RestState>,
            ))
            .layer(oidc_login_service)
            .layer(oidc_auth_service)
            .layer(session_layer)
            .layer(CookieManagerLayer::new())
    };

    #[cfg(not(feature = "oidc"))]
    let app = app.layer(CookieManagerLayer::new());

    app
}

pub async fn serve_app(app: Router, listener: tokio::net::TcpListener) {
    axum::serve(listener, app)
        .await
        .expect("Could not start server");
}

pub async fn start_server<RestState: RestStateDef>(rest_state: RestState) {
    let app = create_app(rest_state).await;

    info!("Running server at {}", bind_address());

    let listener = tokio::net::TcpListener::bind(bind_address().as_ref())
        .await
        .expect("Could not bind server");

    serve_app(app, listener).await;
}
