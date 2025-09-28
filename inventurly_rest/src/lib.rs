pub mod person;
pub mod product;
pub mod test_server;

use async_trait::async_trait;
use axum::{body::Body, middleware, response::Response, Router};
use inventurly_service::permission::{Authentication, MockContext};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone, Debug)]
pub struct Context {
    pub auth: Authentication<MockContext>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            auth: Authentication::Context(MockContext),
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

    fn person_service(&self) -> Arc<Self::PersonService>;
    fn product_service(&self) -> Arc<Self::ProductService>;
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/persons", api = person::ApiDoc),
        (path = "/products", api = product::ApiDoc)
    )
)]
pub struct ApiDoc;

pub fn bind_address() -> Arc<str> {
    std::env::var("SERVER_ADDRESS")
        .unwrap_or("0.0.0.0:3000".into())
        .into()
}

async fn add_context<RestState: RestStateDef>(
    mut req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> axum::response::Response {
    req.extensions_mut().insert(Context::default());
    next.run(req).await
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
        .with_state(rest_state.clone())
        .layer(middleware::from_fn(add_context::<RestState>))
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
