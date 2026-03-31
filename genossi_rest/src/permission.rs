use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use genossi_service::{
    auth_types::{
        PrivilegeResponseTO, PrivilegeTO, RolePrivilege, RoleResponseTO, RoleTO, UserResponseTO,
        UserRole, UserTO,
    },
    permission::PermissionService,
    ServiceError,
};
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestError, RestStateDef};

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_users,
        create_user,
        delete_user,
        get_all_roles,
        create_role,
        delete_role,
        get_all_privileges,
        create_privilege,
        delete_privilege,
        assign_user_role,
        remove_user_role,
        get_user_roles,
        assign_role_privilege,
        remove_role_privilege,
        get_role_privileges,
        get_user_privileges,
    ),
    components(
        schemas(UserTO, RoleTO, PrivilegeTO, UserRole, RolePrivilege, UserResponseTO, RoleResponseTO, PrivilegeResponseTO)
    ),
    tags(
        (name = "Permission", description = "User, role, and privilege management endpoints")
    )
)]
pub struct ApiDoc;

// User Management Endpoints

/// Get all users
#[utoipa::path(
    get,
    path = "/user",
    tags = ["Permission"],
    responses(
        (status = 200, description = "List of all users", body = [UserResponseTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn get_all_users<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            let users = permission_service
                .get_all_users(auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Json(users.as_ref()).into_response())
        })
        .await,
    )
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/user",
    tags = ["Permission"],
    request_body = UserTO,
    responses(
        (status = 201, description = "User created successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn create_user<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(user): Json<UserTO>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .create_user(user, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Delete a user
#[utoipa::path(
    delete,
    path = "/user/{username}",
    tags = ["Permission"],
    params(
        ("username" = String, Path, description = "Username to delete")
    ),
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn delete_user<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Path(username): Path<String>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .delete_user(username, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

// Role Management Endpoints

/// Get all roles
#[utoipa::path(
    get,
    path = "/role",
    tags = ["Permission"],
    responses(
        (status = 200, description = "List of all roles", body = [RoleResponseTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn get_all_roles<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            let roles = permission_service
                .get_all_roles(auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Json(roles.as_ref()).into_response())
        })
        .await,
    )
}

/// Create a new role
#[utoipa::path(
    post,
    path = "/role",
    tags = ["Permission"],
    request_body = RoleTO,
    responses(
        (status = 201, description = "Role created successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn create_role<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(role): Json<RoleTO>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .create_role(role, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Delete a role
#[utoipa::path(
    delete,
    path = "/role/{role_name}",
    tags = ["Permission"],
    params(
        ("role_name" = String, Path, description = "Role name to delete")
    ),
    responses(
        (status = 204, description = "Role deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 404, description = "Role not found"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn delete_role<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Path(role_name): Path<String>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .delete_role(role_name, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

// Privilege Management Endpoints

/// Get all privileges
#[utoipa::path(
    get,
    path = "/privilege",
    tags = ["Permission"],
    responses(
        (status = 200, description = "List of all privileges", body = [PrivilegeResponseTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn get_all_privileges<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            let privileges = permission_service
                .get_all_privileges(auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Json(privileges.as_ref()).into_response())
        })
        .await,
    )
}

/// Create a new privilege
#[utoipa::path(
    post,
    path = "/privilege",
    tags = ["Permission"],
    request_body = PrivilegeTO,
    responses(
        (status = 201, description = "Privilege created successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn create_privilege<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(privilege): Json<PrivilegeTO>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .create_privilege(privilege, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Delete a privilege
#[utoipa::path(
    delete,
    path = "/privilege/{privilege_name}",
    tags = ["Permission"],
    params(
        ("privilege_name" = String, Path, description = "Privilege name to delete")
    ),
    responses(
        (status = 204, description = "Privilege deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 404, description = "Privilege not found"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn delete_privilege<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Path(privilege_name): Path<String>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .delete_privilege(privilege_name, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

// User-Role Relationship Management

/// Assign a role to a user
#[utoipa::path(
    post,
    path = "/user-role",
    tags = ["Permission"],
    request_body = UserRole,
    responses(
        (status = 201, description = "Role assigned to user successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn assign_user_role<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(user_role): Json<UserRole>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .assign_user_role(user_role, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Remove a role from a user
#[utoipa::path(
    delete,
    path = "/user-role",
    tags = ["Permission"],
    request_body = UserRole,
    responses(
        (status = 204, description = "Role removed from user successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn remove_user_role<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(user_role): Json<UserRole>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .remove_user_role(user_role, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Get roles assigned to a user
#[utoipa::path(
    get,
    path = "/user/{username}/roles",
    tags = ["Permission"],
    params(
        ("username" = String, Path, description = "Username to get roles for")
    ),
    responses(
        (status = 200, description = "List of roles for the user", body = [RoleResponseTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn get_user_roles<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Path(username): Path<String>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            let roles = permission_service
                .get_user_roles(username, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Json(roles.as_ref()).into_response())
        })
        .await,
    )
}

// Role-Privilege Relationship Management

/// Assign a privilege to a role
#[utoipa::path(
    post,
    path = "/role-privilege",
    tags = ["Permission"],
    request_body = RolePrivilege,
    responses(
        (status = 201, description = "Privilege assigned to role successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn assign_role_privilege<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(role_privilege): Json<RolePrivilege>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .assign_role_privilege(role_privilege, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Remove a privilege from a role
#[utoipa::path(
    delete,
    path = "/role-privilege",
    tags = ["Permission"],
    request_body = RolePrivilege,
    responses(
        (status = 204, description = "Privilege removed from role successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn remove_role_privilege<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Extension(context): Extension<Context>,
    Json(role_privilege): Json<RolePrivilege>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            permission_service
                .remove_role_privilege(role_privilege, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(axum::body::Body::empty())
                .unwrap())
        })
        .await,
    )
}

/// Get privileges assigned to a role
#[utoipa::path(
    get,
    path = "/role/{role_name}/privileges",
    tags = ["Permission"],
    params(
        ("role_name" = String, Path, description = "Role name to get privileges for")
    ),
    responses(
        (status = 200, description = "List of privileges for the role", body = [PrivilegeResponseTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 404, description = "Role not found"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn get_role_privileges<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Path(role_name): Path<String>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            let privileges = permission_service
                .get_role_privileges(role_name, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Json(privileges.as_ref()).into_response())
        })
        .await,
    )
}

/// Get all privileges assigned to a user (through roles)
#[utoipa::path(
    get,
    path = "/user/{username}/privileges",
    tags = ["Permission"],
    params(
        ("username" = String, Path, description = "Username to get privileges for")
    ),
    responses(
        (status = 200, description = "List of privileges for the user", body = [PrivilegeResponseTO]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin privilege required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(rest_state))]
pub async fn get_user_privileges<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    Path(username): Path<String>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let permission_service = rest_state.permission_service();
            let auth_context = crate::extract_auth_context(Some(context))?;

            let privileges = permission_service
                .get_user_privileges(username, auth_context)
                .await
                .map_err(|e| service_error_to_rest_error(e))?;

            Ok(Json(privileges.as_ref()).into_response())
        })
        .await,
    )
}

// Helper functions

// Use the global extract_auth_context helper from lib.rs

/// Convert ServiceError to RestError
fn service_error_to_rest_error(error: ServiceError) -> RestError {
    match error {
        ServiceError::PermissionDenied => RestError::Unauthorized,
        ServiceError::Unauthorized => RestError::Unauthorized,
        ServiceError::EntityNotFound(_) => RestError::NotFound,
        ServiceError::ValidationError(errors) => {
            let msg = errors
                .iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join(", ");
            RestError::BadRequest(msg)
        }
        ServiceError::DataAccess(msg) => RestError::InternalError(msg.to_string()),
        ServiceError::InternalError(msg) => RestError::InternalError(msg.to_string()),
        ServiceError::SessionExpired => RestError::Unauthorized,
        ServiceError::AuthenticationFailed => RestError::Unauthorized,
    }
}

/// Generate router for permission management endpoints
pub fn generate_route<RestState: RestStateDef>() -> axum::Router<RestState> {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        // User management
        .route("/user", get(get_all_users::<RestState>))
        .route("/user", post(create_user::<RestState>))
        .route("/user/{username}", delete(delete_user::<RestState>))
        // Role management
        .route("/role", get(get_all_roles::<RestState>))
        .route("/role", post(create_role::<RestState>))
        .route("/role/{role_name}", delete(delete_role::<RestState>))
        // Privilege management
        .route("/privilege", get(get_all_privileges::<RestState>))
        .route("/privilege", post(create_privilege::<RestState>))
        .route(
            "/privilege/{privilege_name}",
            delete(delete_privilege::<RestState>),
        )
        // User-Role relationships
        .route("/user-role", post(assign_user_role::<RestState>))
        .route("/user-role", delete(remove_user_role::<RestState>))
        .route("/user/{username}/roles", get(get_user_roles::<RestState>))
        // Role-Privilege relationships
        .route("/role-privilege", post(assign_role_privilege::<RestState>))
        .route(
            "/role-privilege",
            delete(remove_role_privilege::<RestState>),
        )
        .route(
            "/role/{role_name}/privileges",
            get(get_role_privileges::<RestState>),
        )
        // User privileges (computed)
        .route(
            "/user/{username}/privileges",
            get(get_user_privileges::<RestState>),
        )
}
