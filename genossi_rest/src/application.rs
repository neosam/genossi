use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_dao::application::ApplicationStatus;
use genossi_rest_types::{ApplicationStatusTO, ApplicationTO, PublicJoinRequest, PublicJoinResponse, SalutationTO};
use genossi_service::application::{ApplicationService, ApplicationSubmission};
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait ApplicationRestState: Clone + Send + Sync + 'static {
    type ApplicationService: ApplicationService<Context = crate::ContextType> + Send + Sync + 'static;

    fn application_service(&self) -> Arc<Self::ApplicationService>;
    fn get_config_value(
        &self,
        key: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>>;
}

#[derive(Debug, serde::Deserialize)]
pub struct ApplicationListQuery {
    pub status: Option<String>,
}

// --- Public endpoint (no auth, API key required) ---

#[instrument(skip(state, headers))]
#[utoipa::path(
    post,
    tag = "Public Join",
    path = "/join",
    request_body = PublicJoinRequest,
    responses(
        (status = 201, description = "Application submitted", body = PublicJoinResponse),
        (status = 401, description = "Invalid or missing API key"),
        (status = 422, description = "Validation error"),
    ),
)]
pub async fn public_join<S: ApplicationRestState>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(body): Json<PublicJoinRequest>,
) -> Response {
    error_handler(
        (async {
            // Validate API key
            let api_key = headers
                .get("X-Api-Key")
                .and_then(|v| v.to_str().ok())
                .ok_or(RestError::Unauthorized)?;

            let stored_key = state
                .get_config_value("public_api_key")
                .await
                .ok_or(RestError::Unauthorized)?;

            if api_key != stored_key {
                return Err(RestError::Unauthorized);
            }

            let salutation = body.salutation.as_ref().map(genossi_dao::member::Salutation::from);

            let submission = ApplicationSubmission {
                first_name: Arc::from(body.first_name.as_str()),
                last_name: Arc::from(body.last_name.as_str()),
                salutation,
                email: Arc::from(body.email.as_str()),
                street: Arc::from(body.street.as_str()),
                house_number: Arc::from(body.house_number.as_str()),
                postal_code: Arc::from(body.postal_code.as_str()),
                city: Arc::from(body.city.as_str()),
                shares: body.shares,
            };

            state.application_service().submit(&submission).await?;

            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::to_string(&PublicJoinResponse {
                        message: "Beitrittserklärung eingegangen".to_string(),
                    })
                    .unwrap(),
                ))
                .unwrap())
        })
        .await,
    )
}

pub fn generate_public_route<S: ApplicationRestState>() -> Router<S> {
    Router::new().route("/join", post(public_join::<S>))
}

// --- Admin endpoints (auth required) ---

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Applications",
    path = "",
    params(
        ("status" = Option<String>, Query, description = "Filter by status (Offen, Bestaetigt, Abgelehnt)")
    ),
    responses(
        (status = 200, description = "List applications", body = [ApplicationTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_applications<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<ApplicationListQuery>,
) -> Response {
    error_handler(
        (async {
            let status_filter = query.status.as_deref().map(|s| match s {
                "Offen" => Ok(ApplicationStatus::Offen),
                "Bestaetigt" => Ok(ApplicationStatus::Bestaetigt),
                "Abgelehnt" => Ok(ApplicationStatus::Abgelehnt),
                other => Err(RestError::BadRequest(format!("Unknown status: {}", other))),
            }).transpose()?;

            let apps: Arc<[ApplicationTO]> = rest_state
                .application_service()
                .list(status_filter, crate::extract_auth_context(Some(context))?)
                .await?
                .iter()
                .map(ApplicationTO::from)
                .collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&apps).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Applications",
    path = "/{id}",
    responses(
        (status = 200, description = "Get application", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
)]
pub async fn get_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let app = rest_state
                .application_service()
                .get(id, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(&app)).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Applications",
    path = "/{id}/confirm",
    responses(
        (status = 200, description = "Application confirmed, member created", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict - application not in Offen status"),
    ),
)]
pub async fn confirm_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let app = rest_state
                .application_service()
                .confirm(id, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(&app)).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Applications",
    path = "/{id}/reject",
    responses(
        (status = 200, description = "Application rejected", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict - application not in Offen status"),
    ),
)]
pub async fn reject_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let app = rest_state
                .application_service()
                .reject(id, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(&app)).unwrap()))
                .unwrap())
        })
        .await,
    )
}

pub fn generate_route<RestState: RestStateDef + ApplicationRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_applications::<RestState>))
        .route("/{id}", get(get_application::<RestState>))
        .route("/{id}/confirm", post(confirm_application::<RestState>))
        .route("/{id}/reject", post(reject_application::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(list_applications, get_application, confirm_application, reject_application),
    components(schemas(ApplicationTO, ApplicationStatusTO, PublicJoinResponse))
)]
pub struct ApiDoc;

#[derive(OpenApi)]
#[openapi(
    paths(public_join),
    components(schemas(PublicJoinRequest, PublicJoinResponse))
)]
pub struct PublicApiDoc;
