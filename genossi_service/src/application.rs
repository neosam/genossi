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
    pub title: Option<Arc<str>>,
    pub email: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
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
            title: entity.title.clone(),
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
            title: app.title.clone(),
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

/// Input for submitting a new application.
#[derive(Clone, Debug)]
pub struct ApplicationSubmission {
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Salutation>,
    pub title: Option<Arc<str>>,
    pub email: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
    pub shares: i32,
}

/// Phase 31 (APMAIL-01/02, D-03): raw content for a manual applicant mail send.
///
/// The service stamps exactly ONE recipient (`application.email`) around this
/// content — `subject`/`body`/`body_html` still carry unrendered placeholders;
/// the worker resolves them per-recipient via the shared render kernel. These
/// types are deliberately `genossi_service`-local (NOT `genossi_mail` types):
/// `genossi_mail` depends on `genossi_service`, so re-importing a `genossi_mail`
/// type here would create a dependency cycle.
#[derive(Clone, Debug)]
pub struct ApplicationMailInput {
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
    pub template_id: Option<Uuid>,
}

/// Phase 31 (D-06): preview request — the draft an admin wants to see rendered
/// before sending. No recipient, no template_id (preview never enqueues).
#[derive(Clone, Debug)]
pub struct ApplicationMailDraft {
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
}

/// Phase 31 (D-06): preview result — the rendered subject/body/body_html the
/// admin sees. Cycle-free (no `genossi_mail::RenderedContent`).
#[derive(Clone, Debug)]
pub struct RenderedApplicationMail {
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,
}

/// Input for updating an existing application.
#[derive(Clone, Debug)]
pub struct ApplicationUpdate {
    pub first_name: Arc<str>,
    pub last_name: Arc<str>,
    pub salutation: Option<Salutation>,
    pub title: Option<Arc<str>>,
    pub email: Option<Arc<str>>,
    pub street: Option<Arc<str>>,
    pub house_number: Option<Arc<str>>,
    pub postal_code: Option<Arc<str>>,
    pub city: Option<Arc<str>>,
    pub shares: i32,
    pub version: Uuid,
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait ApplicationService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    /// Submit a new application (public or admin, no auth context needed).
    /// Creates the application and optionally triggers the confirmation mail.
    async fn submit(
        &self,
        submission: &ApplicationSubmission,
        send_mail: bool,
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

    /// Update an existing application's fields (requires manage_members).
    async fn update_application(
        &self,
        id: Uuid,
        update: &ApplicationUpdate,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<Application, ServiceError>;

    /// Phase 31 (APMAIL-01/02, APCMP-01/02): enqueue a manual applicant mail.
    ///
    /// CR-02 ordering guarantee (identical to `confirm`): the permission check
    /// runs FIRST, then NotFound, then the `Offen`-only status guard — no
    /// user-attributable side effect precedes the permission check. Unlike the
    /// `send_confirmation_mail` anti-pattern, this method NEVER returns a silent
    /// `()`: every synchronous failure (403 PermissionDenied, 404 EntityNotFound,
    /// 409 Conflict on status ≠ Offen, 400 ValidationError on missing address,
    /// 500 InternalError on enqueue failure) is propagated as a real error.
    /// Exactly ONE application-bound recipient (`application.email`) is stamped;
    /// there is no mass-send path and no free-text recipient.
    async fn send_mail(
        &self,
        id: Uuid,
        input: &ApplicationMailInput,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    /// Phase 31 (D-06): render a draft through the SAME pure render kernel the
    /// worker uses (`render_application_content`), so preview output is
    /// byte-identical to what the recipient receives. Permission check runs
    /// FIRST, then NotFound. Status-independent (no `Offen` guard) and never
    /// enqueues — pure preview.
    async fn preview_mail(
        &self,
        id: Uuid,
        draft: &ApplicationMailDraft,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<RenderedApplicationMail, ServiceError>;

    /// Phase 31 (APHIST-02, D-07 Option A): server-side anti-double-send guard.
    /// Returns `MAX(entry.date)` over `get_application_communications(id)` —
    /// where `date = COALESCE(sent_at, created)` — or `None` if the applicant
    /// has no outbound history. Permission check runs FIRST.
    async fn last_sent_at(
        &self,
        id: Uuid,
        context: crate::permission::Authentication<Self::Context>,
    ) -> Result<Option<time::PrimitiveDateTime>, ServiceError>;
}
