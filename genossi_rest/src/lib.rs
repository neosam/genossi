pub mod application;
pub mod assembly;
pub mod attendance;
pub mod attendance_export;
pub mod repayment_export;
// Phase 13 (D-13-01..11): REST-Handler fuer Bulk-PDF-Anschreiben (Vorstand-only).
pub mod audit_log;
pub mod audit_timestamp;
pub mod auth;
pub mod auth_middleware;
pub mod backup;
#[cfg(debug_assertions)]
pub mod dev;
pub mod helper_token;
pub mod http_util;
pub mod mail_footer;
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod membership_adjust;
pub mod permission;
pub mod public_stats;
pub mod repayment_entry;
pub mod repayment_letter;
pub mod repayment_phase;
pub mod session;
pub mod session_management;
pub mod static_document;
pub mod template;
pub mod test_server;
pub mod user_preference;
pub mod validation;

use async_trait::async_trait;
use axum::routing::get;
use axum::{body::Body, middleware, response::Response, Router};
#[cfg(feature = "oidc")]
use genossi_service::auth_types::AuthenticatedContext;
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::permission::MockContext;
use http::{header, Method};
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "oidc")]
use axum::response::IntoResponse;
use axum::response::Redirect;

// Simplified context type to match shifty pattern - just the user ID
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub type Context = MockContext;
#[cfg(feature = "oidc")]
pub type Context = Option<genossi_service::auth_types::AuthenticatedContext>;

// Helper function to extract Authentication from simplified Context
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub fn extract_auth_context(
    context: Option<Context>,
) -> Result<genossi_service::permission::Authentication<MockContext>, RestError> {
    match context {
        Some(ctx) => Ok(genossi_service::permission::Authentication::Context(ctx)),
        None => Err(RestError::Unauthorized),
    }
}

#[cfg(feature = "oidc")]
pub fn extract_auth_context(
    context: Option<Context>,
) -> Result<
    genossi_service::permission::Authentication<genossi_service::auth_types::AuthenticatedContext>,
    RestError,
> {
    match context {
        Some(Some(auth_context)) => Ok(genossi_service::permission::Authentication::Context(
            auth_context,
        )),
        _ => Err(RestError::Unauthorized),
    }
}

pub enum RestError {
    NotFound,
    BadRequest(String),
    Conflict(String),
    Unauthorized,
    UnsupportedMediaType(String),
    InternalError(String),
    /// 403 Forbidden — used by helper-redeem when token is revoked or assembly not Open (D-24).
    Forbidden(String),
    /// 410 Gone — used by helper-redeem when token has already been used (D-24).
    Gone(String),
}

impl From<serde_json::Error> for RestError {
    fn from(e: serde_json::Error) -> Self {
        RestError::InternalError(format!("serialize failed: {}", e))
    }
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
            genossi_mail::service::MailServiceError::TemplateValidation(msg) => {
                RestError::BadRequest(msg.to_string())
            }
            genossi_mail::service::MailServiceError::BadRequest(msg) => {
                RestError::BadRequest(msg.to_string())
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
        Err(RestError::UnsupportedMediaType(msg)) => Response::builder()
            .status(415)
            .header("Content-Type", "application/json")
            .body(Body::from(msg))
            .unwrap(),
        // D-24: 403 Forbidden + 410 Gone for helper-redeem differential mapping.
        Err(RestError::Forbidden(msg)) => Response::builder()
            .status(403)
            .body(Body::from(msg))
            .unwrap(),
        Err(RestError::Gone(msg)) => Response::builder()
            .status(410)
            .body(Body::from(msg))
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
pub trait RestStateDef:
    Clone
    + Send
    + Sync
    + 'static
    + genossi_config::rest::ConfigRestState
    + genossi_mail::rest::MailRestState
    + genossi_mail::rest_templates::MailTemplateRestState
    + genossi_mail::inbox_rest::InboxRestState
    + genossi_mail::communication_rest::CommunicationRestState
{
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
    // Phase 15 v1.2 (D-15-16): MembershipAdjustService — Foundation fuer
    // cancel_membership + increase_shares (Phase 16-17 erweitern um
    // partial_repayment + transfer_shares).
    type MembershipAdjustService: genossi_service::membership_adjust::MembershipAdjustService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type DocumentStorage: genossi_service::document_storage::DocumentStorage + Send + Sync + 'static;
    type ValidationService: genossi_service::validation::ValidationService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type UserPreferenceService: genossi_service::user_preference::UserPreferenceService<Context = ContextType>
        + Send
        + Sync
        + 'static;
    type StaticDocumentService: genossi_mail::static_document_service::StaticDocumentService
        + Send
        + Sync
        + 'static;
    type BackupDao: genossi_dao::backup::BackupDao + Send + Sync + 'static;

    fn member_service(&self) -> Arc<Self::MemberService>;
    fn permission_service(&self) -> Arc<Self::PermissionService>;
    fn session_service(&self) -> Arc<Self::SessionService>;
    fn member_import_service(&self) -> Arc<Self::MemberImportService>;
    fn member_action_service(&self) -> Arc<Self::MemberActionService>;
    fn member_document_service(&self) -> Arc<Self::MemberDocumentService>;
    fn membership_adjust_service(&self) -> Arc<Self::MembershipAdjustService>;
    fn document_storage(&self) -> Arc<Self::DocumentStorage>;
    fn validation_service(&self) -> Arc<Self::ValidationService>;
    fn user_preference_service(&self) -> Arc<Self::UserPreferenceService>;
    fn static_document_service(&self) -> Arc<Self::StaticDocumentService>;
    fn template_storage(&self) -> Arc<genossi_service_impl::template_storage::TemplateStorage>;
    fn pdf_generator(&self) -> Arc<genossi_service_impl::pdf_generation::PdfGenerator>;
    fn backup_dao(&self) -> Arc<Self::BackupDao>;
}

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/api/auth", api = auth::ApiDoc),
        (path = "/api/members", api = member::ApiDoc),
        (path = "/api/members", api = membership_adjust::ApiDoc),
        (path = "/api/members/{member_id}/actions", api = member_action::ApiDoc),
        (path = "/api/members/{member_id}/documents", api = member_document::ApiDoc),
        (path = "/api/permission", api = permission::ApiDoc),
        (path = "/api/validation", api = validation::ApiDoc),
        (path = "/api/templates", api = template::ApiDoc),
        (path = "/api/user-preferences", api = user_preference::ApiDoc),
        (path = "/api/config", api = genossi_config::rest::ApiDoc),
        (path = "/api/mail", api = genossi_mail::rest::ApiDoc),
        (path = "/api/inbox", api = genossi_mail::inbox_rest::InboxApiDoc),
        (path = "/api/members/{member_id}/communications", api = genossi_mail::communication_rest::CommunicationApiDoc),
        (path = "/api/static-documents", api = static_document::ApiDoc),
        (path = "/api/mail/footer", api = mail_footer::ApiDoc),
        (path = "/api/member-documents", api = member_document::CountsApiDoc),
        (path = "/api/applications", api = application::ApiDoc),
        (path = "/api/assembly", api = assembly::ApiDoc),
        (path = "/api/repayment-phase", api = repayment_phase::ApiDoc),
        (path = "/api/repayment-entry", api = repayment_entry::ApiDoc),
        (path = "/api/assembly/{assembly_id}/helper-tokens", api = helper_token::ApiDoc),
        (path = "/api/attendance/{assembly_id}", api = attendance::ApiDoc),
        (path = "/api/assembly/{assembly_id}/attendance-export", api = attendance_export::ApiDoc),
        (path = "/api/repayment-phase/{phase_id}/export", api = repayment_export::ApiDoc),
        (path = "/api/repayment-phase/{phase_id}/letters", api = repayment_letter::ApiDoc),
        (path = "/api/audit", api = audit_log::ApiDoc),
        (path = "/api/audit/timestamps", api = audit_timestamp::ApiDoc),
        (path = "/api/session", api = session_management::ApiDoc)
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

fn build_cors_layer(cors_allowed_origins: Option<&str>) -> CorsLayer {
    let base = std::env::var("BASE_PATH").unwrap_or_else(|_| "http://localhost:3000".into());
    // Strip trailing slash for origin matching
    let base_origin = base.trim_end_matches('/').to_string();

    let mut origins: Vec<http::HeaderValue> = Vec::new();

    // Always include BASE_PATH origin
    match http::HeaderValue::from_str(&base_origin) {
        Ok(val) => origins.push(val),
        Err(e) => tracing::warn!("Invalid BASE_PATH origin '{}': {}", base_origin, e),
    }

    // Add configured additional origins
    if let Some(extra) = cors_allowed_origins {
        for origin_str in extra.split(',') {
            let trimmed = origin_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            match http::HeaderValue::from_str(trimmed) {
                Ok(val) => origins.push(val),
                Err(e) => tracing::warn!("Invalid CORS origin '{}': {}", trimmed, e),
            }
        }
    }

    tracing::info!("CORS allowed origins: {:?}", origins);

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
        // WR-03 fix: Browser blendet alle custom Response-Header standardmaessig
        // bei cross-origin requests aus — sie muessen explizit via Access-Control-
        // Expose-Headers freigegeben werden, sonst liest das Frontend `None`.
        //
        // - x-document-count (Phase 13 D-13-04): Frontend nutzt den Header fuer
        //   Toast-Pluralisierung "N Briefe erzeugt" nach Aggregation. Ohne expose
        //   greift der Fallback `entry_ids.len()` und das Toast-Wording wird falsch.
        // - content-disposition: enthaelt den Filename fuer Browser-Save. Heute
        //   wird der Filename clientseitig konstruiert, daher latent — wir
        //   exponieren ihn trotzdem fuer kuenftige Direct-Save-Pfade.
        .expose_headers([
            http::HeaderName::from_static("x-document-count"),
            http::HeaderName::from_static("content-disposition"),
        ])
}

fn apply_security_headers(router: Router) -> Router {
    router
        .layer(SetResponseHeaderLayer::if_not_present(
            http::header::STRICT_TRANSPORT_SECURITY,
            http::HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            http::header::X_CONTENT_TYPE_OPTIONS,
            http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            http::header::X_FRAME_OPTIONS,
            http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            http::header::REFERRER_POLICY,
            http::HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            http::HeaderName::from_static("permissions-policy"),
            http::HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
        ))
}

pub async fn create_app<
    RestState: RestStateDef
        + public_stats::PublicStatsState
        + application::ApplicationRestState
        + assembly::AssemblyRestState
        + repayment_phase::RepaymentPhaseRestState
        + repayment_entry::RepaymentEntryRestState
        + helper_token::HelperTokenRestState
        + attendance::AttendanceRestState
        + attendance_export::AttendanceExportRestState
        + repayment_export::RepaymentExportRestState
        + repayment_letter::RepaymentLetterRestState
        + audit_log::AuditRestState
        + audit_timestamp::TimestampRestState,
>(
    rest_state: RestState,
) -> Router {
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

    let public_stats_doc = public_stats::ApiDoc::openapi();
    api_doc.merge(public_stats_doc);

    let public_join_doc = application::PublicApiDoc::openapi();
    api_doc.merge(public_join_doc);

    // Plan 02-07: Helper-Redeem PublicApiDoc (D-22 public flow).
    let helper_redeem_doc = helper_token::PublicApiDoc::openapi();
    api_doc.merge(helper_redeem_doc);

    let swagger_router = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_doc);

    // Read CORS config at startup
    let cors_allowed_origins = rest_state.get_config_value("cors_allowed_origins").await;
    let cors_layer = build_cors_layer(cors_allowed_origins.as_deref());

    // Rate-limiting configs (token bucket)
    use tower_governor::governor::GovernorConfigBuilder;
    use tower_governor::GovernorLayer;

    // 10 req/min on /authenticate
    let auth_rate_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(6)
            .burst_size(10)
            .finish()
            .unwrap(),
    );
    let auth_rate_layer = GovernorLayer {
        config: auth_rate_config,
    };

    // 60 req/min on /api/* (global API limit)
    let api_rate_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(60)
            .finish()
            .unwrap(),
    );
    let api_rate_layer = GovernorLayer {
        config: api_rate_config,
    };

    // 5 req/min on /api/public/join
    let join_rate_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(12)
            .burst_size(5)
            .finish()
            .unwrap(),
    );
    let join_rate_layer = GovernorLayer {
        config: join_rate_config,
    };

    // ~10 req/min on /api/helper/redeem (Plan 02-07, RESEARCH Pitfall 7).
    // Brute-force protection on the public helper-redeem endpoint.
    let redeem_rate_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(6) // 60s window / 6s per request = 10/min steady-state
            .burst_size(10) // allow short bursts for re-tries
            .finish()
            .unwrap(),
    );
    let redeem_rate_layer = GovernorLayer {
        config: redeem_rate_config,
    };

    let app = Router::new().route("/authenticate", get(login));

    #[cfg(feature = "oidc")]
    let app = {
        use axum::error_handling::HandleErrorLayer;
        use axum_oidc::error::MiddlewareError;
        use axum_oidc::{EmptyAdditionalClaims, OidcLoginLayer};

        use tower::ServiceBuilder;

        let oidc_login_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|e: MiddlewareError| async {
                tracing::error!("OIDC Login error: {:?}", e);
                e.into_response()
            }))
            .layer(OidcLoginLayer::<EmptyAdditionalClaims>::new());
        app.layer(oidc_login_service)
    };

    // Rate limit on /authenticate (10/min per IP)
    let app = app.layer(auth_rate_layer);

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
        .nest(
            "/api/templates/render-application",
            template::generate_render_application_route(),
        )
        // Quick 260603-kon: Test-Route fuer Repayment-Letter-Templates mit
        // Dummy-Sentinel-Werten. Strikt OFF-Pfad — kein Audit, kein
        // MemberDocument-Insert. Siehe Doc-Comment in template.rs:render_repayment_letter_test.
        .nest(
            "/api/templates/render-repayment-test",
            template::generate_render_repayment_test_route(),
        )
        .nest("/api/user-preferences", user_preference::generate_route())
        .nest("/api/config", genossi_config::rest::generate_route())
        .nest("/api/mail", genossi_mail::rest::generate_route())
        .nest(
            "/api/mail/templates",
            genossi_mail::rest_templates::generate_route::<RestState>(),
        )
        .nest(
            "/api/mail/footer",
            mail_footer::generate_route::<RestState>(),
        )
        .nest(
            "/api/inbox",
            genossi_mail::inbox_rest::generate_route::<RestState>(),
        )
        .nest(
            "/api/members/{member_id}/communications",
            genossi_mail::communication_rest::generate_route::<RestState>(),
        )
        .nest(
            "/api/member-documents",
            member_document::generate_counts_route::<RestState>(),
        )
        .nest(
            "/api/static-documents",
            static_document::generate_route::<RestState>(),
        )
        .nest("/api/backup", backup::generate_route::<RestState>())
        .nest(
            "/api/applications",
            application::generate_route::<RestState>(),
        )
        .nest("/api/assembly", assembly::generate_route::<RestState>())
        .nest(
            "/api/repayment-phase",
            repayment_phase::generate_route::<RestState>(),
        )
        .nest(
            "/api/repayment-entry",
            repayment_entry::generate_route::<RestState>(),
        )
        .nest(
            "/api/assembly/{assembly_id}/helper-tokens",
            helper_token::generate_route::<RestState>(),
        )
        // Phase 3 Plan 06 (D-21): attendance live-counter under the assembly
        // namespace because the counter is semantically an assembly aspect,
        // even though the implementation lives in AttendanceService (D-23).
        .nest(
            "/api/assembly/{assembly_id}",
            attendance::generate_stats_route::<RestState>(),
        )
        // Phase 3 Plan 06 (D-21): attendance toggle + reduced member list.
        .nest(
            "/api/attendance/{assembly_id}",
            attendance::generate_attendance_route::<RestState>(),
        )
        // Phase 6 Plan 03 (D-14): Teilnehmerlisten-Export fuer geschlossene GVs.
        // Mounted unter /api/assembly (nicht /api/attendance), weil der Export
        // ein Aggregat-Operation auf der Assembly ist (Filename, Status-Gate,
        // Permission-Funnel kommen aus dem Assembly-Kontext).
        .nest(
            "/api/assembly",
            attendance_export::generate_export_route::<RestState>(),
        )
        // Phase 11 Plan 04 (D-12, D-03, D-11): PDF-Export der Auszahlungsliste.
        // Mounted unter /api/repayment-phase — Axum 0.8.3 merged das mit dem
        // bereits existierenden repayment_phase::generate_route() unter dem
        // gleichen Prefix; die Pfade /{phase_id} und /{phase_id}/export/{format}
        // kollidieren nicht (unique segments).
        .nest(
            "/api/repayment-phase",
            repayment_export::generate_export_route::<RestState>(),
        )
        // Phase 13 Plan 05 (D-13-02/03/04): Bulk-PDF-Anschreiben via
        // POST /{phase_id}/letters/generate. Mounted unter /api/repayment-phase
        // — Axum 0.8.3 merged das mit den existierenden Mounts unter dem
        // gleichen Prefix; /{phase_id}/letters/generate ist disjunkt zu
        // /{phase_id} (RepaymentPhase) und /{phase_id}/export/{format}.
        .nest(
            "/api/repayment-phase",
            repayment_letter::generate_letter_route::<RestState>(),
        )
        .nest("/api/audit", audit_log::generate_route::<RestState>())
        .nest(
            "/api/audit/timestamps",
            audit_timestamp::generate_route::<RestState>(),
        )
        .nest(
            "/api/session",
            session_management::generate_route::<RestState>(),
        )
        .with_state(rest_state.clone())
        .layer(middleware::from_fn_with_state(
            rest_state.clone(),
            session::forbid_unauthenticated::<RestState>,
        ))
        .layer(middleware::from_fn_with_state(
            rest_state.clone(),
            context_extractor::<RestState>,
        ))
        .layer(api_rate_layer)
        .layer(cors_layer);

    #[cfg(feature = "oidc")]
    let app = {
        use axum::error_handling::HandleErrorLayer;
        use axum_oidc::error::MiddlewareError;
        use axum_oidc::{EmptyAdditionalClaims, OidcAuthLayer};
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

    // Public routes (no auth required)
    let join_router = application::generate_public_route::<RestState>().layer(join_rate_layer);
    let helper_redeem_router =
        helper_token::generate_public_route::<RestState>().layer(redeem_rate_layer);
    let app = app
        .nest("/api/public", public_stats::generate_route::<RestState>())
        .nest("/api/public", join_router)
        .nest("/api/helper", helper_redeem_router)
        .with_state(rest_state.clone());

    // Dev-only routes (no auth required, only compiled in debug builds)
    #[cfg(debug_assertions)]
    let app = app
        .nest("/api/dev", dev::generate_route::<RestState>())
        .with_state(rest_state.clone());

    // Security headers on all routes
    apply_security_headers(app)
}

pub async fn serve_app(app: Router, listener: tokio::net::TcpListener) {
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("Could not start server");
}

pub async fn start_server<
    RestState: RestStateDef
        + public_stats::PublicStatsState
        + application::ApplicationRestState
        + assembly::AssemblyRestState
        + repayment_phase::RepaymentPhaseRestState
        + repayment_entry::RepaymentEntryRestState
        + helper_token::HelperTokenRestState
        + attendance::AttendanceRestState
        + attendance_export::AttendanceExportRestState
        + repayment_export::RepaymentExportRestState
        + repayment_letter::RepaymentLetterRestState
        + audit_log::AuditRestState
        + audit_timestamp::TimestampRestState,
>(
    rest_state: RestState,
) {
    let app = create_app(rest_state).await;

    info!("Running server at {}", bind_address());

    let listener = tokio::net::TcpListener::bind(bind_address().as_ref())
        .await
        .expect("Could not bind server");

    serve_app(app, listener).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_serde_json_error_maps_to_internal_error() {
        let err = serde_json::from_str::<u32>("not a number").unwrap_err();
        let rest_err: RestError = err.into();
        assert!(
            matches!(&rest_err, RestError::InternalError(msg) if msg.contains("serialize failed")),
            "expected RestError::InternalError with 'serialize failed', got something else"
        );
    }

    #[test]
    fn test_helper_variant_compiles_in_both_features() {
        // Smoke test: AuthContext::Helper kann in diesem Crate konstruiert werden,
        // ohne cfg-Annotation. Wenn Cargo die Variante hinter einem Feature-Flag
        // versteckt, bricht dieser Test in dem Build, der das Feature nicht hat.
        // Schützt den D-14-Vertrag (keine cfg-Gate auf Helper-Variante).
        use genossi_service::auth_types::AuthContext;
        use std::sync::Arc;
        let _ = AuthContext::Helper {
            session_id: Arc::from("test"),
            assembly_id: uuid::Uuid::nil(),
        };
    }
}
