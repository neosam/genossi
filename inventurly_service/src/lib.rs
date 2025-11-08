pub mod auth_types;
pub mod container;
pub mod csv_import;
pub mod duplicate_detection;
pub mod inventur;
pub mod inventur_custom_entry;
pub mod inventur_measurement;
pub mod permission;
pub mod person;
pub mod product;
pub mod product_rack;
pub mod rack;
pub mod session;
pub mod user_service;
pub mod uuid_service;

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

impl From<inventurly_dao::DaoError> for ServiceError {
    fn from(e: inventurly_dao::DaoError) -> Self {
        match e {
            inventurly_dao::DaoError::NotFound => ServiceError::EntityNotFound(uuid::Uuid::nil()),
            _ => ServiceError::DataAccess(Arc::from(format!("{:?}", e))),
        }
    }
}
