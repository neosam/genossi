pub mod auth;
pub mod auth_middleware;
#[cfg(debug_assertions)]
pub mod dev;
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod permission;
pub mod session;
pub mod template;
pub mod test_server;
pub mod validation;

use async_trait::async_trait;
use axum::routing::get;
use axum::{body::Body, middleware, response::IntoResponse, response::Response, Router};
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::permission::MockContext;
#[cfg(feature = "oidc")]
use genossi_service::auth_types::AuthenticatedContext;
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use axum::response::Redirect;

// Simplified context type to match shifty pattern - just the user ID
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub type Context = MockContext;
#[cfg(feature = "oidc")]
pub type Context = Option<genossi_service::auth_types::AuthenticatedContext>;

// Helper function to extract Authentication from simplified Context
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub fn extract_auth_context(context: Option<Context>) -> Result<genossi_service::permission::Authentication<MockContext>, RestError> {
    match context {
        Some(ctx) => Ok(genossi_service::permission::Authentication::Context(ctx)),
        None => Err(RestError::Unauthorized),
    }
}

#[cfg(feature = "oidc")]
pub fn extract_auth_context(context: Option<Context>) -> Result<genossi_service::permission::Authentication<genossi_service::auth_types::AuthenticatedContext>, RestError> {
    match context {
        Some(Some(auth_context)) => {
            Ok(genossi_service::permission::Authentication::Context(auth_context))
        }
        _ => Err(RestError::Unauthorized),
    }
}


pub enum RestError {
    NotFound,
    BadRequest(String),
    Conflict(String),
    Unauthorized,
    InternalError(String),
}

impl From<genossi_service::ServiceError> for RestError {
    fn from(e: genossi_service::ServiceError) -> Self {
        match e {
            genossi_service::ServiceError::EntityNotFound(_) => RestError::NotFound,
            genossi_service::ServiceError::ValidationError(items) => {
                let messages: Vec<String> = items
                    .iter()
                    .map(|i| format!("{}: {}", i.field, i.message))
                    .collect();
                RestError::BadRequest(messages.join(", "))
            }
            genossi_service::ServiceError::PermissionDenied => RestError::Unauthorized,
            genossi_service::ServiceError::Conflict(msg) => RestError::Conflict(msg.to_string()),
            _ => RestError::InternalError(format!("{:?}", e)),
        }
    }
}

impl From<genossi_mail::service::MailServiceError> for RestError {
    fn from(e: genossi_mail::service::MailServiceError) -> Self {
        match e {
            genossi_mail::service::MailServiceError::NotFound => RestError::NotFound,
            genossi_mail::service::MailServiceError::DataAccess(msg) => {
                RestError::InternalError(msg.to_string())
            }
            genossi_mail::service::MailServiceError::ConfigMissing(msg) => {
                RestError::BadRequest(msg.to_string())
            }
            genossi_mail::service::MailServiceError::SmtpError(msg) => {
                RestError::InternalError(msg.to_string())
            }
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
        Err(RestError::Conflict(msg)) => Response::builder()
            .status(409)
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

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
type ContextType = MockContext;
#[cfg(feature = "oidc")]
type ContextType = AuthenticatedContext;

#[async_trait]
pub trait RestStateDef: Clone + Send + Sync + 'static + genossi_config::rest::ConfigRestState + genossi_mail::rest::MailRestState {
    type MemberService: genossi_service::member::MemberService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type PermissionService: genossi_service::permission::PermissionService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type SessionService: genossi_service::session::SessionService + Send + Sync + 'static;
    type MemberImportService: genossi_service::member_import::MemberImportService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type MemberActionService: genossi_service::member_action::MemberActionService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type MemberDocumentService: genossi_service::member_document::MemberDocumentService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type DocumentStorage: genossi_service::document_storage::DocumentStorage + Send + Sync + 'static;
    type ValidationService: genossi_service::validation::ValidationService<Context = ContextType>
        + Send
        + Sync
        + 'static;

    fn member_service(&self) -> Arc<Self::MemberService>;
    fn permission_service(&self) -> Arc<Self::PermissionService>;
    fn session_service(&self) -> Arc<Self::SessionService>;
    fn member_import_service(&self) -> Arc<Self::MemberImportService>;
    fn member_action_service(&self) -> Arc<Self::MemberActionService>;
    fn member_document_service(&self) -> Arc<Self::MemberDocumentService>;
    fn document_storage(&self) -> Arc<Self::DocumentStorage>;
    fn validation_service(&self) -> Arc<Self::ValidationService>;
    fn template_storage(&self) -> Arc<genossi_service_impl::template_storage::TemplateStorage>;
    fn pdf_generator(&self) -> Arc<genossi_service_impl::pdf_generation::PdfGenerator>;
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/api/auth", api = auth::ApiDoc),
        (path = "/api/members", api = member::ApiDoc),
        (path = "/api/members/{member_id}/actions", api = member_action::ApiDoc),
        (path = "/api/members/{member_id}/documents", api = member_document::ApiDoc),
        (path = "/api/permission", api = permission::ApiDoc),
        (path = "/api/validation", api = validation::ApiDoc),
        (path = "/api/templates", api = template::ApiDoc),
        (path = "/api/config", api = genossi_config::rest::ApiDoc),
        (path = "/api/mail", api = genossi_mail::rest::ApiDoc)
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
    tracing::info!(
        "  CLIENT_SECRET: {}",
        if client_secret.is_some() {
            "***PROVIDED***"
        } else {
            "NOT_SET"
        }
    );

    let filtered_secret = client_secret.filter(|s| !s.is_empty());
    if filtered_secret.is_none() {
        tracing::warn!(
            "CLIENT_SECRET is empty or not set - this may cause authentication failures"
        );
    }

    OidcConfig {
        app_url,
        issuer,
        client_id,
        client_secret: filtered_secret,
    }
}

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
        .description(Some("Genossi backend"))
        .build()]);

    #[cfg(debug_assertions)]
    {
        let dev_doc = dev::api_doc();
        api_doc.merge(dev_doc);
    }

    let swagger_router = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_doc);

    let app = Router::new().route("/authenticate", get(login));
    
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

        let oidc_login_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
                tracing::error!("OIDC Login error: {:?}", e);
                e.into_response()
            }))
            .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());
        app.layer(oidc_login_service)
    };

    #[allow(unused_mut)]
    let mut app = app.merge(swagger_router);

    let app = app
        .nest("/api/auth", auth::generate_route())
        .nest("/api/members", member::generate_route())
        .nest(
            "/api/members/{member_id}/actions",
            member_action::generate_route(),
        )
        .nest(
            "/api/members/{member_id}/documents",
            member_document::generate_route(),
        )
        .nest("/api/permission", permission::generate_route())
        .nest("/api/validation", validation::generate_route())
        .nest("/api/templates", template::generate_route())
        .nest("/api/templates/render", template::generate_render_route())
        .nest("/api/config", genossi_config::rest::generate_route())
        .nest("/api/mail", genossi_mail::rest::generate_route())
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
            }
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
        app.route("/logout", get(logout))
            .layer(middleware::from_fn_with_state(
                rest_state.clone(),
                session::register_session::<RestState>,
            ))
            .layer(oidc_auth_service)
            .layer(session_layer)
            .layer(CookieManagerLayer::new())
    };

    #[cfg(not(feature = "oidc"))]
    let app = app.layer(CookieManagerLayer::new());

    // Dev-only routes (no auth required, only compiled in debug builds)
    #[cfg(debug_assertions)]
    let app = app
        .nest("/api/dev", dev::generate_route::<RestState>())
        .with_state(rest_state.clone());

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
