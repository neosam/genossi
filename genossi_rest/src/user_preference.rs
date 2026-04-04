use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::{get, put},
    Extension, Json, Router,
};
use genossi_rest_types::UserPreferenceTO;
use genossi_service::user_preference::UserPreferenceService;
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/{key}", get(get_preference::<RestState>))
        .route("/{key}", put(upsert_preference::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "User Preferences",
    path = "/{key}",
    params(("key" = String, Path, description = "Preference key")),
    responses(
        (status = 200, description = "Get user preference", body = UserPreferenceTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Preference not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_preference<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(key): Path<String>,
) -> Response {
    error_handler(
        (async {
            let pref = rest_state
                .user_preference_service()
                .get_by_key(&key, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let to = UserPreferenceTO::from(&pref);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "User Preferences",
    path = "/{key}",
    params(("key" = String, Path, description = "Preference key")),
    request_body = UserPreferenceTO,
    responses(
        (status = 200, description = "Upsert user preference", body = UserPreferenceTO),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn upsert_preference<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(key): Path<String>,
    Json(body): Json<UserPreferenceTO>,
) -> Response {
    error_handler(
        (async {
            let pref = rest_state
                .user_preference_service()
                .upsert(&key, &body.value, crate::extract_auth_context(Some(context))?, None)
                .await?;
            let to = UserPreferenceTO::from(&pref);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_preference,
        upsert_preference
    ),
    components(schemas(UserPreferenceTO)),
    tags((name = "User Preferences", description = "Per-user preference management endpoints"))
)]
pub struct ApiDoc;
