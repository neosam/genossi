pub mod person;
pub mod product;
pub mod csv_import;
pub mod permission;
pub mod uuid_service;
pub mod user_service;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ServiceError {
    DataAccess(Arc<str>),
    EntityNotFound(uuid::Uuid),
    ValidationError(Vec<ValidationFailureItem>),
    PermissionDenied,
    InternalError(Arc<str>),
}

#[derive(Debug, Clone)]
pub struct ValidationFailureItem {
    pub field: Arc<str>,
    pub message: Arc<str>,
}

impl From<inventurly_dao::DaoError> for ServiceError {
    fn from(e: inventurly_dao::DaoError) -> Self {
        match e {
            inventurly_dao::DaoError::NotFound => ServiceError::EntityNotFound(uuid::Uuid::nil()),
            _ => ServiceError::DataAccess(Arc::from(format!("{:?}", e))),
        }
    }
}