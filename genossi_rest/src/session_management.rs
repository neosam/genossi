use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use genossi_service::{
    permission::{Authentication, PermissionService, ADMIN_PRIVILEGE},
    session::SessionService,
};
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestError, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        revoke_all_sessions,
        admin_revoke_user_sessions,
    ),
    components(
        schemas(genossi_rest_types::SessionRevokeResponse)
    ),
    tags(
        (name = "Session Management", description = "Session management endpoints")
    )
)]
pub struct ApiDoc;

/// Revoke all sessions for the current user
///
/// Deletes all active sessions for the currently authenticated user,
/// including the session used for this request. The user will need to
/// log in again after this call.
#[utoipa::path(
    post,
    path = "/revoke-all",
    tags = ["Session Management"],
    responses(
        (status = 200, description = "All sessions revoked", body = genossi_rest_types::SessionRevokeResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
#[instrument(skip(rest_state, context))]
pub async fn revoke_all_sessions<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(revoke_all_sessions_impl(rest_state, context).await)
}

async fn revoke_all_sessions_impl<RestState: RestStateDef>(
    rest_state: RestState,
    context: Context,
) -> Result<Response, RestError> {
    let user_id = extract_user_id(&context)?;

    let revoked_count = rest_state
        .session_service()
        .revoke_all_for_user(&user_id)
        .await
        .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

    Ok(Json(genossi_rest_types::SessionRevokeResponse {
        message: "Alle Sessions beendet.".to_string(),
        revoked_count,
    })
    .into_response())
}

#[cfg(feature = "oidc")]
fn extract_user_id(context: &Context) -> Result<String, RestError> {
    match context {
        Some(auth_context) => Ok(auth_context.user_id.to_string()),
        None => Err(RestError::Unauthorized),
    }
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
fn extract_user_id(_context: &Context) -> Result<String, RestError> {
    Ok("DEVUSER".to_string())
}

/// Revoke all sessions for a specific user (admin only)
///
/// Deletes all active sessions for the specified user. Requires admin privileges.
/// Use this when revoking a user's access, e.g. after removing their permissions.
#[utoipa::path(
    post,
    path = "/revoke/{user_id}",
    tags = ["Session Management"],
    params(
        ("user_id" = String, Path, description = "The user ID whose sessions to revoke")
    ),
    responses(
        (status = 200, description = "User sessions revoked", body = genossi_rest_types::SessionRevokeResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin privilege required"),
    )
)]
#[instrument(skip(rest_state, context))]
pub async fn admin_revoke_user_sessions<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Path(user_id): Path<String>,
) -> Response {
    error_handler(admin_revoke_user_sessions_impl(rest_state, context, user_id).await)
}

async fn admin_revoke_user_sessions_impl<RestState: RestStateDef>(
    rest_state: RestState,
    context: Context,
    user_id: String,
) -> Result<Response, RestError> {
    // Require admin privilege
    let auth = crate::extract_auth_context(Some(context))?;
    let authentication: Authentication<_> = auth;
    rest_state
        .permission_service()
        .check_permission(ADMIN_PRIVILEGE, authentication)
        .await
        .map_err(RestError::from)?;

    let revoked_count = rest_state
        .session_service()
        .revoke_all_for_user(&user_id)
        .await
        .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

    Ok(Json(genossi_rest_types::SessionRevokeResponse {
        message: format!("Sessions für {} beendet.", user_id),
        revoked_count,
    })
    .into_response())
}

pub fn generate_route<RestState: RestStateDef>() -> axum::Router<RestState> {
    use axum::routing::post;
    axum::Router::new()
        .route("/revoke-all", post(revoke_all_sessions::<RestState>))
        .route(
            "/revoke/{user_id}",
            post(admin_revoke_user_sessions::<RestState>),
        )
}
