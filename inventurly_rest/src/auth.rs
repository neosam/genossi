use axum::{
    Extension, Json, body::Body, extract::State, response::{IntoResponse, Response}
};
use inventurly_service::inventur::InventurService;
use inventurly_service::permission::PermissionService;
use inventurly_service::session::SessionService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestError, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_auth_info,
        inventur_token_login,
    ),
    components(
        schemas(AuthInfoResponse, InventurTokenLoginRequest, InventurTokenLoginResponse)
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
    pub claims: HashMap<String, String>,
}

/// Request for inventur token login
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InventurTokenLoginRequest {
    #[schema(example = "abc123xyz789...")]
    pub token: String,
    #[schema(example = "John Doe")]
    pub name: String,
}

/// Response for inventur token login
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InventurTokenLoginResponse {
    pub success: bool,
    pub message: String,
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
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    {
        let username = "DEVUSER".to_string();
        let auth = inventurly_service::permission::Authentication::Context(context);
        
        // Get actual roles and privileges from permission service
        let permission_service = rest_state.permission_service();
        
        // Get user's roles
        let roles = match permission_service
            .get_user_roles(username.clone(), auth.clone())
            .await
        {
            Ok(roles) => roles.iter().map(|r| r.name.to_string()).collect(),
            Err(_) => vec![], // If we can't get roles, return empty list
        };
        
        // Get user's privileges
        let privileges = match permission_service
            .get_user_privileges(username.clone(), auth.clone())
            .await
        {
            Ok(privs) => privs.iter().map(|p| p.name.to_string()).collect(),
            Err(_) => vec![], // If we can't get privileges, return empty list
        };

        let response = AuthInfoResponse {
            username,
            roles,
            privileges,
            claims: HashMap::new(), // Mock context has no claims
        };

        Ok(Json(response).into_response())
    }

    #[cfg(feature = "oidc")]
    {
        match context {
            Some(auth_context) => {
                let username = auth_context.user_id.to_string();

                // Extract claims from auth context, filtering out "type" key
                let claims: HashMap<String, String> = auth_context.claims
                    .as_ref()
                    .and_then(|json_str| serde_json::from_str::<HashMap<String, serde_json::Value>>(json_str).ok())
                    .map(|map| {
                        map.into_iter()
                            .filter(|(key, _)| key != "type") // Filter out metadata
                            .filter_map(|(key, value)| {
                                // Convert JSON value to string
                                match value {
                                    serde_json::Value::String(s) => Some((key, s)),
                                    serde_json::Value::Number(n) => Some((key, n.to_string())),
                                    serde_json::Value::Bool(b) => Some((key, b.to_string())),
                                    _ => None, // Skip complex values
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let auth = inventurly_service::permission::Authentication::Context(auth_context);

                // Get actual roles and privileges from permission service
                let permission_service = rest_state.permission_service();

                // Get user's roles
                let roles = match permission_service
                    .get_user_roles(username.clone(), auth.clone())
                    .await
                {
                    Ok(roles) => roles.iter().map(|r| r.name.to_string()).collect(),
                    Err(_) => vec![], // If we can't get roles, return empty list
                };

                // Get user's privileges
                let privileges = match permission_service
                    .get_user_privileges(username.clone(), auth.clone())
                    .await
                {
                    Ok(privs) => privs.iter().map(|p| p.name.to_string()).collect(),
                    Err(_) => vec![], // If we can't get privileges, return empty list
                };

                let response = AuthInfoResponse {
                    username,
                    roles,
                    privileges,
                    claims,
                };

                Ok(Json(response).into_response())
            }
            None => Err(RestError::Unauthorized),
        }
    }
}

/// Login using inventur token
///
/// Accepts an inventur token and user name, creates a scoped session for inventory counting.
/// The session is limited to operations on the specific inventur associated with the token.
#[utoipa::path(
    post,
    path = "/inventur-token",
    tags = ["Authentication"],
    request_body = InventurTokenLoginRequest,
    responses(
        (status = 200, description = "Successfully logged in", body = InventurTokenLoginResponse),
        (status = 401, description = "Invalid token or inventur not active"),
        (status = 400, description = "Bad request"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn inventur_token_login<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Json(request): Json<InventurTokenLoginRequest>,
) -> Response {
    error_handler(inventur_token_login_impl(rest_state, request).await)
}

async fn inventur_token_login_impl<RestState: RestStateDef>(
    rest_state: RestState,
    request: InventurTokenLoginRequest,
) -> Result<Response, RestError> {
    // Look up inventur by token (no authentication required for this)
    let inventur = rest_state
        .inventur_service()
        .find_by_token(&request.token, None)
        .await?
        .ok_or_else(|| RestError::Unauthorized)?;

    // Check if inventur is active
    if inventur.status.as_ref() != "active" {
        return Err(RestError::Unauthorized);
    }

    // Sanitize the name
    let sanitized_name = request.name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>();

    // Create user_id
    let user_id = format!("inventur-token:{}:{}", inventur.id, sanitized_name);

    // Create claims JSON
    let claims = serde_json::json!({
        "inventur_id": inventur.id.to_string(),
        "type": "inventur_token"
    }).to_string();

    // Create session with claims (24 hours)
    // This will auto-create the virtual user if it doesn't exist
    let session = rest_state
        .session_service()
        .ensure_user_and_create_session_with_claims(&user_id, 24 * 60 * 60, Some(claims))
        .await?;

    let response = InventurTokenLoginResponse {
        success: true,
        message: format!("Logged in for inventur: {}. Session ID: {}", inventur.name, session.session_id),
    };

    //Ok(Json(response).into_response())
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("Set-Cookie", format!("app_session={}; HttpOnly; Secure; Path=/; Max-Age=86400", session.session_id))
        .body(Body::new(serde_json::to_string(&response).unwrap()))
        .unwrap())
}

pub fn generate_route<RestState: RestStateDef>() -> axum::Router<RestState> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/info", get(get_auth_info::<RestState>))
        .route("/inventur-token", post(inventur_token_login::<RestState>))
}
