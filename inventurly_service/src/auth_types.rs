use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::PrimitiveDateTime;

/// Transfer object for user creation requests
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserTO {
    pub name: String,
}

/// Transfer object for role creation requests
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RoleTO {
    pub name: String,
}

/// Transfer object for privilege creation requests
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PrivilegeTO {
    pub name: String,
}

/// Transfer object for user-role relationship operations
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserRole {
    pub user: String,
    pub role: String,
}

/// Transfer object for role-privilege relationship operations
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RolePrivilege {
    pub role: String,
    pub privilege: String,
}

/// Response transfer object for user data
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserResponseTO {
    pub name: String,
    pub update_timestamp: Option<PrimitiveDateTime>,
    pub update_process: String,
}

/// Response transfer object for role data
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RoleResponseTO {
    pub name: String,
    pub update_timestamp: Option<PrimitiveDateTime>,
    pub update_process: String,
}

/// Response transfer object for privilege data
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PrivilegeResponseTO {
    pub name: String,
    pub update_timestamp: Option<PrimitiveDateTime>,
    pub update_process: String,
}

/// Authentication mode configuration
#[derive(Debug, Clone)]
pub enum AuthMode {
    /// Mock authentication for development
    Mock,
    /// OpenID Connect authentication for production
    #[cfg(feature = "oidc")]
    Oidc(OidcConfig),
}

/// OIDC configuration structure
#[cfg(feature = "oidc")]
#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub app_url: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Arc<str>,
}

/// Session information for authenticated users
#[derive(Debug, Clone)]
pub struct UserSession {
    pub session_id: Arc<str>,
    pub user_id: Arc<str>,
    pub expires_at: i64,
    pub created_at: i64,
}

/// Authentication context for different modes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthContext {
    /// Mock context for development
    Mock(MockContext),
    /// OIDC context with user ID
    #[cfg(feature = "oidc")]
    Oidc(Arc<str>),
}

/// Mock authentication context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockContext {
    pub user_id: Arc<str>,
}

impl Default for MockContext {
    fn default() -> Self {
        Self {
            user_id: "DEVUSER".into(),
        }
    }
}

/// Standard privilege names
pub mod privileges {
    pub const ADMIN: &str = "admin";
    pub const USER: &str = "user";
    pub const READONLY: &str = "readonly";
}

/// Standard role names
pub mod roles {
    pub const ADMIN: &str = "admin";
    pub const USER: &str = "user";
    pub const READONLY: &str = "readonly";
}

/// Default development users
pub mod dev_users {
    pub const DEVUSER: &str = "DEVUSER";
    pub const ADMIN: &str = "admin";
}
