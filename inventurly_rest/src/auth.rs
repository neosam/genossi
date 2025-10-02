use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestError, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_auth_info,
    ),
    components(
        schemas(AuthInfoResponse)
    ),
    tags(
        (name = "Authentication", description = "Authentication and authorization endpoints")
    )
)]
pub struct ApiDoc;

/// Response for current user authentication info
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuthInfoResponse {
    pub username: String,
    pub roles: Vec<String>,
}

/// Get current user authentication info
///
/// Returns information about the currently authenticated user including their roles.
/// Returns 401 if not authenticated.
#[utoipa::path(
    get,
    path = "/info",
    tags = ["Authentication"],
    responses(
        (status = 200, description = "Successfully retrieved auth info", body = AuthInfoResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
#[instrument(skip(_rest_state, context))]
pub async fn get_auth_info<RestState: RestStateDef>(
    State(_rest_state): State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(get_auth_info_impl(context).await)
}

async fn get_auth_info_impl(context: Context) -> Result<Response, RestError> {
    match context.auth_context {
        Some(auth_context) => {
            // Extract user information from auth context
            // For now, using mock data - this should be replaced with actual auth context data
            let response = AuthInfoResponse {
                username: "test_user".to_string(), // TODO: Extract from auth_context
                roles: vec!["admin".to_string(), "user".to_string()], // TODO: Extract from auth_context
            };
            
            Ok(Json(response).into_response())
        }
        None => Err(RestError::Unauthorized),
    }
}

pub fn generate_route<RestState: RestStateDef>() -> axum::Router<RestState> {
    use axum::routing::get;
    
    axum::Router::new()
        .route("/info", get(get_auth_info::<RestState>))
}