pub mod auth_types;
pub mod template;
pub mod claim_context;
pub mod claim_utils;
pub mod document_storage;
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod member_import;
pub mod permission;
pub mod session;
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
    Unauthorized,
    SessionExpired,
    AuthenticationFailed,
}

#[derive(Debug, Clone)]
pub struct ValidationFailureItem {
    pub field: Arc<str>,
    pub message: Arc<str>,
}

impl From<genossi_dao::DaoError> for ServiceError {
    fn from(e: genossi_dao::DaoError) -> Self {
        match e {
            genossi_dao::DaoError::NotFound => ServiceError::EntityNotFound(uuid::Uuid::nil()),
            _ => ServiceError::DataAccess(Arc::from(format!("{:?}", e))),
        }
    }
}
