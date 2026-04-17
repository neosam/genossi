pub mod application;
pub mod auth_types;
pub mod claim_context;
pub mod claim_utils;
pub mod document_storage;
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod member_import;
pub mod permission;
pub mod session;
pub mod template;
pub mod timestamp;
pub mod user_preference;
pub mod user_service;
pub mod uuid_service;
pub mod validation;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ServiceError {
    DataAccess(Arc<str>),
    EntityNotFound(uuid::Uuid),
    ValidationError(Vec<ValidationFailureItem>),
    PermissionDenied,
    InternalError(Arc<str>),
    Conflict(Arc<str>),
    Unauthorized,
    SessionExpired,
    AuthenticationFailed,
}

#[derive(Debug, Clone)]
pub struct ValidationFailureItem {
    pub field: Arc<str>,
    pub message: Arc<str>,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::DataAccess(msg) => write!(f, "data access error: {msg}"),
            ServiceError::EntityNotFound(id) => write!(f, "entity not found: {id}"),
            ServiceError::ValidationError(items) => {
                write!(f, "validation error: ")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", item.field, item.message)?;
                }
                Ok(())
            }
            ServiceError::PermissionDenied => write!(f, "permission denied"),
            ServiceError::InternalError(msg) => write!(f, "internal error: {msg}"),
            ServiceError::Conflict(msg) => write!(f, "conflict: {msg}"),
            ServiceError::Unauthorized => write!(f, "unauthorized"),
            ServiceError::SessionExpired => write!(f, "session expired"),
            ServiceError::AuthenticationFailed => write!(f, "authentication failed"),
        }
    }
}

impl From<genossi_dao::DaoError> for ServiceError {
    fn from(e: genossi_dao::DaoError) -> Self {
        match e {
            genossi_dao::DaoError::NotFound => ServiceError::EntityNotFound(uuid::Uuid::nil()),
            _ => ServiceError::DataAccess(Arc::from(format!("{:?}", e))),
        }
    }
}
