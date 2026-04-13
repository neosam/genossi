use async_trait::async_trait;
use genossi_dao::application::{ApplicationEntity, ApplicationStatus};
use genossi_dao::member::Salutation;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Application {
    pub id: Uuid,
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Salutation>,
    pub email: Arc<str>,
    pub street: Arc<str>,
    pub house_number: Arc<str>,
    pub postal_code: Arc<str>,
    pub city: Arc<str>,
    pub shares: i32,
    pub status: ApplicationStatus,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&ApplicationEntity> for Application {
    fn from(entity: &ApplicationEntity) -> Self {
        Self {
            id: entity.id,
            first_name: entity.first_name.clone(),
            last_name: entity.last_name.clone(),
            salutation: entity.salutation.clone(),
            email: entity.email.clone(),
            street: entity.street.clone(),
            house_number: entity.house_number.clone(),
            postal_code: entity.postal_code.clone(),
            city: entity.city.clone(),
            shares: entity.shares,
            status: entity.status.clone(),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Application> for ApplicationEntity {
    fn from(app: &Application) -> Self {
        Self {
            id: app.id,
            first_name: app.first_name.clone(),
            last_name: app.last_name.clone(),
            salutation: app.salutation.clone(),
            email: app.email.clone(),
            street: app.street.clone(),
            house_number: app.house_number.clone(),
            postal_code: app.postal_code.clone(),
            city: app.city.clone(),
            shares: app.shares,
            status: app.status.clone(),
            created: app.created,
            deleted: app.deleted,
            version: app.version,
        }
    }
}

/// Input for submitting a new application (public endpoint).
#[derive(Clone, Debug)]
pub struct ApplicationSubmission {
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Salutation>,
    pub email: Arc<str>,
    pub street: Arc<str>,
    pub house_number: Arc<str>,
    pub postal_code: Arc<str>,
    pub city: Arc<str>,
    pub shares: i32,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait ApplicationService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Submit a new application (public, no auth context needed).
    /// Creates the application and triggers the confirmation mail.
    async fn submit(
        &self,
        submission: &ApplicationSubmission,
    ) -> Result<Application, ServiceError>;

    /// List applications with optional status filter (requires manage_members).
    async fn list(
        &self,
        status_filter: Option<ApplicationStatus>,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<Arc<[Application]>, ServiceError>;

    /// Get a single application by ID (requires manage_members).
    async fn get(
        &self,
        id: Uuid,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<Application, ServiceError>;

    /// Confirm an application: creates a member and sets status to Bestaetigt.
    async fn confirm(
        &self,
        id: Uuid,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<Application, ServiceError>;

    /// Reject an application: sets status to Abgelehnt.
    async fn reject(
        &self,
        id: Uuid,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<Application, ServiceError>;
}
