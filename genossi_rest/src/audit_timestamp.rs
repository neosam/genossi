use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::get,
    Extension, Router,
};
use genossi_rest_types::{
    TimestampCreateResponseTO, TimestampResponseTO, TimestampVerifyResponseTO,
};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::timestamp::{TimestampError, TimestampService};
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait TimestampRestState: RestStateDef {
    type TimestampService: TimestampService + Send + Sync + 'static;

    fn timestamp_service(&self) -> std::sync::Arc<Self::TimestampService>;
}

pub fn generate_route<RestState: TimestampRestState>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            get(list_timestamps::<RestState>).post(create_timestamp::<RestState>),
        )
        .route("/{id}/verify", get(verify_timestamp::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(list_timestamps, create_timestamp, verify_timestamp),
    components(schemas(
        TimestampResponseTO,
        TimestampVerifyResponseTO,
        TimestampCreateResponseTO
    )),
    tags((name = "Audit Timestamps", description = "Qualified timestamp management and verification"))
)]
pub struct ApiDoc;

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Audit Timestamps",
    path = "",
    responses(
        (status = 200, description = "List of all timestamps", body = Vec<TimestampResponseTO>),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_timestamps<RestState: TimestampRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let authentication: Authentication<_> = Authentication::from(auth);
            rest_state
                .permission_service()
                .check_permission("admin", authentication)
                .await?;

            let entries = rest_state
                .timestamp_service()
                .get_all()
                .await
                .map_err(|e| RestError::InternalError(format!("{}", e)))?;

            let result: Vec<TimestampResponseTO> =
                entries.iter().map(TimestampResponseTO::from).collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Audit Timestamps",
    path = "",
    responses(
        (status = 201, description = "Timestamp created", body = TimestampCreateResponseTO),
        (status = 200, description = "No changes to timestamp", body = TimestampCreateResponseTO),
        (status = 401, description = "Unauthorized"),
        (status = 502, description = "TSA unreachable"),
    ),
)]
pub async fn create_timestamp<RestState: TimestampRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let authentication: Authentication<_> = Authentication::from(auth);
            rest_state
                .permission_service()
                .check_permission("admin", authentication)
                .await?;

            match rest_state.timestamp_service().create_timestamp().await {
                Ok(entry) => {
                    let response = TimestampCreateResponseTO {
                        created: true,
                        message: "Timestamp created successfully".to_string(),
                        timestamp: Some(TimestampResponseTO::from(&entry)),
                    };
                    Ok(Response::builder()
                        .status(201)
                        .header("Content-Type", "application/json")
                        .body(Body::new(serde_json::to_string(&response)?))
                        .unwrap())
                }
                Err(TimestampError::DuplicateHash) => {
                    let response = TimestampCreateResponseTO {
                        created: false,
                        message: "No changes since last timestamp".to_string(),
                        timestamp: None,
                    };
                    Ok(Response::builder()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(Body::new(serde_json::to_string(&response)?))
                        .unwrap())
                }
                Err(TimestampError::NothingToTimestamp) => {
                    let response = TimestampCreateResponseTO {
                        created: false,
                        message: "No audit entries exist".to_string(),
                        timestamp: None,
                    };
                    Ok(Response::builder()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(Body::new(serde_json::to_string(&response)?))
                        .unwrap())
                }
                Err(TimestampError::NotConfigured) => {
                    Err(RestError::BadRequest("TSA not configured".to_string()))
                }
                Err(TimestampError::TsaError(e)) => {
                    Err(RestError::InternalError(format!("TSA error: {}", e)))
                }
                Err(e) => Err(RestError::InternalError(format!("{}", e))),
            }
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Audit Timestamps",
    path = "/{id}/verify",
    params(
        ("id" = Uuid, Path, description = "Timestamp UUID"),
    ),
    responses(
        (status = 200, description = "Verification result", body = TimestampVerifyResponseTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Timestamp not found"),
    ),
)]
pub async fn verify_timestamp<RestState: TimestampRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let authentication: Authentication<_> = Authentication::from(auth);
            rest_state
                .permission_service()
                .check_permission("admin", authentication)
                .await?;

            let verification =
                rest_state
                    .timestamp_service()
                    .verify(id)
                    .await
                    .map_err(|e| match e {
                        TimestampError::NotFound => RestError::NotFound,
                        _ => RestError::InternalError(format!("{}", e)),
                    })?;

            let result = TimestampVerifyResponseTO::from(&verification);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result)?))
                .unwrap())
        })
        .await,
    )
}
