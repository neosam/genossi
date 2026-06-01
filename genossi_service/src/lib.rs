pub mod application;
pub mod assembly;
pub mod attendance;
pub mod attendance_export;
pub mod auth_types;
pub mod claim_context;
pub mod claim_utils;
pub mod document_storage;
pub mod helper_token;
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod member_import;
pub mod permission;
pub mod repayment_context;
pub mod repayment_entry;
pub mod repayment_export;
pub mod repayment_phase;
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
            genossi_dao::DaoError::ConflictError(msg) => ServiceError::Conflict(msg),
            _ => ServiceError::DataAccess(Arc::from(format!("{:?}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_dao::DaoError;

    #[test]
    fn dao_conflict_error_maps_to_service_conflict() {
        let dao_err = DaoError::ConflictError(Arc::from("Version mismatch"));
        let svc_err: ServiceError = dao_err.into();
        match svc_err {
            ServiceError::Conflict(msg) => assert_eq!(msg.as_ref(), "Version mismatch"),
            other => panic!("expected ServiceError::Conflict, got {:?}", other),
        }
    }

    #[test]
    fn dao_not_found_maps_to_service_entity_not_found() {
        let dao_err = DaoError::NotFound;
        let svc_err: ServiceError = dao_err.into();
        match svc_err {
            ServiceError::EntityNotFound(id) => assert_eq!(id, uuid::Uuid::nil()),
            other => panic!("expected ServiceError::EntityNotFound, got {:?}", other),
        }
    }

    #[test]
    fn dao_database_error_maps_to_service_data_access() {
        let dao_err = DaoError::DatabaseError(Arc::from("connection refused"));
        let svc_err: ServiceError = dao_err.into();
        match svc_err {
            ServiceError::DataAccess(_) => {}
            other => panic!("expected ServiceError::DataAccess, got {:?}", other),
        }
    }
}
