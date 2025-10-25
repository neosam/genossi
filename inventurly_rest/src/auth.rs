use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Extension, Json,
};
use inventurly_service::permission::PermissionService;
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
    pub privileges: Vec<String>,
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
#[instrument(skip(rest_state, context))]
pub async fn get_auth_info<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(get_auth_info_impl(rest_state, context).await)
}

async fn get_auth_info_impl<RestState: RestStateDef>(
    rest_state: RestState,
    context: Context,
) -> Result<Response, RestError> {
    match context.auth_context {
        Some(auth_context) => {
            // Extract user information from auth context
            let username = match auth_context {
                #[cfg(feature = "mock_auth")]
                inventurly_service::auth_types::AuthContext::Mock(ref mock_ctx) => {
                    mock_ctx.user_id.to_string()
                }
                #[cfg(feature = "oidc")]
                inventurly_service::auth_types::AuthContext::Oidc(ref user_id) => {
                    user_id.to_string()
                }
            };

            // Get actual roles and privileges from permission service
            let permission_service = rest_state.permission_service();
            
            // Get user's roles
            let roles = match permission_service
                .get_user_roles(username.clone(), context.auth.clone())
                .await
            {
                Ok(roles) => roles.iter().map(|r| r.name.to_string()).collect(),
                Err(_) => vec![], // If we can't get roles, return empty list
            };
            
            // Get user's privileges
            let privileges = match permission_service
                .get_user_privileges(username.clone(), context.auth.clone())
                .await
            {
                Ok(privs) => privs.iter().map(|p| p.name.to_string()).collect(),
                Err(_) => vec![], // If we can't get privileges, return empty list
            };

            let response = AuthInfoResponse {
                username,
                roles,
                privileges,
            };

            Ok(Json(response).into_response())
        }
        None => Err(RestError::Unauthorized),
    }
}

pub fn generate_route<RestState: RestStateDef>() -> axum::Router<RestState> {
    use axum::routing::get;

    axum::Router::new().route("/info", get(get_auth_info::<RestState>))
}
