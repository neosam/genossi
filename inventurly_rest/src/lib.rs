pub mod person;
pub mod product;
pub mod csv_import;
pub mod duplicate_detection;
pub mod test_server;
pub mod auth_middleware;
pub mod permission;

use async_trait::async_trait;
use axum::{body::Body, middleware, response::Response, Router};
use inventurly_service::{
    permission::{Authentication, MockContext},
    auth_types::AuthContext,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone, Debug)]
pub struct Context {
    pub auth: Authentication<MockContext>,
    pub auth_context: Option<AuthContext>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            auth: Authentication::Context(MockContext),
            auth_context: Some(AuthContext::Mock(inventurly_service::auth_types::MockContext::default())),
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
    type SessionService: inventurly_service::session::SessionService
        + Send
        + Sync
        + 'static;

    fn person_service(&self) -> Arc<Self::PersonService>;
    fn product_service(&self) -> Arc<Self::ProductService>;
    fn csv_import_service(&self) -> Arc<Self::CsvImportService>;
    fn duplicate_detection_service(&self) -> Arc<Self::DuplicateDetectionService>;
    fn permission_service(&self) -> Arc<Self::PermissionService>;
    fn session_service(&self) -> Arc<Self::SessionService>;
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/persons", api = person::ApiDoc),
        (path = "/products", api = product::ApiDoc),
        (path = "/csv-import", api = csv_import::CsvImportApiDoc),
        (path = "/duplicate-detection", api = duplicate_detection::DuplicateDetectionApiDoc),
        (path = "/api/permission", api = permission::ApiDoc)
    )
)]
pub struct ApiDoc;

pub fn bind_address() -> Arc<str> {
    std::env::var("SERVER_ADDRESS")
        .unwrap_or("0.0.0.0:3000".into())
        .into()
}

// OIDC takes priority over mock_auth when both are enabled
#[cfg(feature = "oidc")]
async fn context_extractor<RestState: RestStateDef>(
    rest_state: axum::extract::State<RestState>,
    mut request: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    // Extract headers for authentication
    let headers = request.headers().clone();
    
    // Try to extract auth context from headers
    let auth_context = match extract_context_from_headers(&headers, rest_state.session_service().as_ref()).await {
        Ok(Some(ctx)) => Some(ctx),
        Ok(None) => None,
        Err(_) => None,
    };
    
    let context = Context {
        auth: if auth_context.is_some() {
            inventurly_service::permission::Authentication::Context(
                inventurly_service::permission::MockContext
            )
        } else {
            inventurly_service::permission::Authentication::Context(
                inventurly_service::permission::MockContext
            )
        },
        auth_context,
    };
    
    request.extensions_mut().insert(context);
    next.run(request).await
}

#[cfg(all(not(feature = "oidc"), feature = "mock_auth"))]
async fn context_extractor<RestState: RestStateDef>(
    mut request: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    let context = auth_middleware::mock_auth_context();
    request.extensions_mut().insert(context);
    next.run(request).await
}

// Helper function for extracting context from headers 
#[cfg(feature = "oidc")]
async fn extract_context_from_headers<SessionService: inventurly_service::session::SessionService>(
    headers: &axum::http::HeaderMap,
    session_service: &SessionService,
) -> Result<Option<inventurly_service::auth_types::AuthContext>, inventurly_service::ServiceError> {
    // Try session cookie first
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(session_id) = extract_session_from_cookie(cookie_str) {
                if let Some(context) = session_service.extract_auth_context(Some(session_id)).await? {
                    return Ok(Some(context));
                }
            }
        }
    }
    
    // Try Authorization Bearer token (for API access)
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = extract_bearer_token(auth_str) {
                // Treat bearer token as session ID for now
                if let Some(context) = session_service.extract_auth_context(Some(token)).await? {
                    return Ok(Some(context));
                }
            }
        }
    }
    
    Ok(None)
}

#[cfg(feature = "oidc")]
fn extract_session_from_cookie(cookie_str: &str) -> Option<String> {
    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=') {
            if name.trim() == "app_session" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

#[cfg(feature = "oidc")]
fn extract_bearer_token(auth_str: &str) -> Option<String> {
    if auth_str.starts_with("Bearer ") {
        Some(auth_str[7..].to_string())
    } else {
        None
    }
}

pub fn create_app<RestState: RestStateDef>(rest_state: RestState) -> Router {
    let mut api_doc = ApiDoc::openapi();
    let base = std::env::var("BASE_PATH").unwrap_or("http://localhost:3000/".into());
    api_doc.servers = Some(vec![utoipa::openapi::ServerBuilder::new()
        .url(base)
        .description(Some("Inventurly backend"))
        .build()]);

    let swagger_router = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_doc);

    Router::new()
        .merge(swagger_router)
        .nest("/persons", person::generate_route())
        .nest("/products", product::generate_route())
        .nest("/csv-import", csv_import::generate_route())
        .nest("/duplicate-detection", duplicate_detection::generate_route())
        .nest("/api/permission", permission::generate_route())
        .with_state(rest_state.clone())
        .layer(middleware::from_fn_with_state(rest_state.clone(), context_extractor::<RestState>))
        .layer(CorsLayer::permissive())
}

pub async fn serve_app(app: Router, listener: tokio::net::TcpListener) {
    axum::serve(listener, app)
        .await
        .expect("Could not start server");
}

pub async fn start_server<RestState: RestStateDef>(rest_state: RestState) {
    let app = create_app(rest_state);
    
    info!("Running server at {}", bind_address());

    let listener = tokio::net::TcpListener::bind(bind_address().as_ref())
        .await
        .expect("Could not bind server");

    serve_app(app, listener).await;
}
