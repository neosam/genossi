use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::{MailDaoError, MailTemplate, MailTemplateDao};

#[derive(Debug, Clone)]
pub enum MailTemplateError {
    DataAccess(Arc<str>),
    NotFound,
    DuplicateName(Arc<str>),
    VersionConflict,
    BadRequest(Arc<str>),
}

impl From<MailDaoError> for MailTemplateError {
    fn from(e: MailDaoError) -> Self {
        match e {
            MailDaoError::DatabaseError(msg) => MailTemplateError::DataAccess(msg),
            MailDaoError::NotFound => MailTemplateError::NotFound,
        }
    }
}

impl From<serde_json::Error> for MailTemplateError {
    fn from(e: serde_json::Error) -> Self {
        MailTemplateError::DataAccess(Arc::from(format!("serialize failed: {}", e)))
    }
}

#[automock]
#[async_trait]
pub trait MailTemplateService: Send + Sync + 'static {
    /// Phase 23 (HTML-05, D-03 entry point 2):
    /// - `body_html`: optional author HTML sanitized via
    ///   [`crate::sanitize::sanitize_html`] before persistence.
    ///
    /// Note: `Option<String>` (not `Option<&str>`) because `#[automock]` +
    /// `#[async_trait]` can't infer higher-ranked lifetimes on nested
    /// borrowed references in trait methods.
    async fn create(
        &self,
        name: &str,
        subject: &str,
        body: &str,
        body_html: Option<String>,
        // Phase 30 D-01/D-03: Pool-Diskriminator, unveränderlich nach dem Anlegen.
        template_type: &str,
    ) -> Result<MailTemplate, MailTemplateError>;
    /// Phase 23 (HTML-05, D-03 entry point 3):
    /// - `body_html`: optional author HTML sanitized via
    ///   [`crate::sanitize::sanitize_html`] before persistence.
    async fn update(
        &self,
        id: Uuid,
        name: &str,
        subject: &str,
        body: &str,
        body_html: Option<String>,
        version: Uuid,
    ) -> Result<MailTemplate, MailTemplateError>;
    async fn delete(&self, id: Uuid) -> Result<(), MailTemplateError>;
    async fn get(&self, id: Uuid) -> Result<MailTemplate, MailTemplateError>;
    async fn list(&self) -> Result<Arc<[MailTemplate]>, MailTemplateError>;
}

pub struct MailTemplateServiceImpl<D: MailTemplateDao> {
    dao: Arc<D>,
}

impl<D: MailTemplateDao> MailTemplateServiceImpl<D> {
    pub fn new(dao: Arc<D>) -> Self {
        Self { dao }
    }
}

#[async_trait]
impl<D: MailTemplateDao> MailTemplateService for MailTemplateServiceImpl<D> {
    async fn create(
        &self,
        name: &str,
        subject: &str,
        body: &str,
        body_html: Option<String>,
        template_type: &str,
    ) -> Result<MailTemplate, MailTemplateError> {
        if let Some(_existing) = self.dao.find_by_name(name).await? {
            return Err(MailTemplateError::DuplicateName(Arc::from(name)));
        }

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        // Phase 23 D-03 entry point 2 (HTML-05): sanitize before persistence.
        let body_html_sanitized: Option<Arc<str>> = body_html
            .as_deref()
            .map(crate::sanitize::sanitize_html)
            .map(Arc::from);

        let template = MailTemplate {
            id: Uuid::new_v4(),
            created,
            deleted: None,
            version: Uuid::new_v4(),
            name: Arc::from(name),
            subject: Arc::from(subject),
            body: Arc::from(body),
            // Phase 23 D-06: sanitized author HTML (or None for text-only templates).
            body_html: body_html_sanitized,
            // Phase 30 D-01/D-03: Pool-Diskriminator aus dem Create-Body.
            template_type: Arc::from(template_type),
        };

        self.dao.create(&template).await?;
        Ok(template)
    }

    async fn update(
        &self,
        id: Uuid,
        name: &str,
        subject: &str,
        body: &str,
        body_html: Option<String>,
        version: Uuid,
    ) -> Result<MailTemplate, MailTemplateError> {
        let existing = self
            .dao
            .find_by_id(id)
            .await?
            .ok_or(MailTemplateError::NotFound)?;

        if existing.version != version {
            return Err(MailTemplateError::VersionConflict);
        }

        // Check name uniqueness (different template)
        if let Some(other) = self.dao.find_by_name(name).await? {
            if other.id != id {
                return Err(MailTemplateError::DuplicateName(Arc::from(name)));
            }
        }

        // Phase 23 D-03 entry point 3 (HTML-05): sanitize before persistence.
        let body_html_sanitized: Option<Arc<str>> = body_html
            .as_deref()
            .map(crate::sanitize::sanitize_html)
            .map(Arc::from);

        let updated = MailTemplate {
            id: existing.id,
            created: existing.created,
            deleted: None,
            version: Uuid::new_v4(),
            name: Arc::from(name),
            subject: Arc::from(subject),
            body: Arc::from(body),
            // Phase 23 D-06: sanitized author HTML — explicit `None` clears the
            // prior HTML sibling; `Some(...)` replaces it with the newly
            // sanitized content. Update takes full ownership of body_html.
            body_html: body_html_sanitized,
            // Phase 30 D-01: template_type ist nach dem Anlegen unveränderlich
            // (Pitfall 3) — die UPDATE-SQL schreibt die Spalte nicht; die
            // zurückgegebene Entität trägt den bestehenden Wert weiter.
            template_type: existing.template_type.clone(),
        };

        self.dao.update(&updated).await?;
        Ok(updated)
    }

    async fn delete(&self, id: Uuid) -> Result<(), MailTemplateError> {
        let existing = self
            .dao
            .find_by_id(id)
            .await?
            .ok_or(MailTemplateError::NotFound)?;

        let now = time::OffsetDateTime::now_utc();
        let deleted_at = time::PrimitiveDateTime::new(now.date(), now.time());

        let deleted = MailTemplate {
            deleted: Some(deleted_at),
            version: Uuid::new_v4(),
            ..existing
        };

        self.dao.update(&deleted).await?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<MailTemplate, MailTemplateError> {
        self.dao
            .find_by_id(id)
            .await?
            .ok_or(MailTemplateError::NotFound)
    }

    async fn list(&self) -> Result<Arc<[MailTemplate]>, MailTemplateError> {
        Ok(self.dao.all().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dao::MockMailTemplateDao;

    #[tokio::test]
    async fn test_create_success() {
        let mut mock = MockMailTemplateDao::new();
        mock.expect_find_by_name().returning(|_| Ok(None));
        mock.expect_create().returning(|_| Ok(()));

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service
            .create("Test", "Subject", "Body", None, "application")
            .await;
        assert!(result.is_ok());
        let tpl = result.unwrap();
        assert_eq!(tpl.name.as_ref(), "Test");
        assert_eq!(tpl.subject.as_ref(), "Subject");
        assert_eq!(tpl.body.as_ref(), "Body");
        // Phase 30 D-01/D-03: der übergebene template_type wird durchgefädelt.
        assert_eq!(tpl.template_type.as_ref(), "application");
    }

    #[tokio::test]
    async fn test_create_duplicate_name() {
        let mut mock = MockMailTemplateDao::new();
        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        mock.expect_find_by_name().returning(move |_| {
            Ok(Some(MailTemplate {
                id: Uuid::new_v4(),
                created,
                deleted: None,
                version: Uuid::new_v4(),
                name: Arc::from("Existing"),
                subject: Arc::from(""),
                body: Arc::from(""),
                body_html: None,
                template_type: Arc::from("member"),
            }))
        });

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service
            .create("Existing", "Sub", "Body", None, "member")
            .await;
        assert!(matches!(result, Err(MailTemplateError::DuplicateName(_))));
    }

    #[tokio::test]
    async fn test_update_version_conflict() {
        let mut mock = MockMailTemplateDao::new();
        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        let existing_version = Uuid::new_v4();
        mock.expect_find_by_id().returning(move |_| {
            Ok(Some(MailTemplate {
                id: Uuid::new_v4(),
                created,
                deleted: None,
                version: existing_version,
                name: Arc::from("Test"),
                subject: Arc::from(""),
                body: Arc::from(""),
                body_html: None,
                template_type: Arc::from("member"),
            }))
        });

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let wrong_version = Uuid::new_v4();
        let result = service
            .update(Uuid::new_v4(), "Test", "Sub", "Body", None, wrong_version)
            .await;
        assert!(matches!(result, Err(MailTemplateError::VersionConflict)));
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let mut mock = MockMailTemplateDao::new();
        mock.expect_find_by_id().returning(|_| Ok(None));

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service.delete(Uuid::new_v4()).await;
        assert!(matches!(result, Err(MailTemplateError::NotFound)));
    }

    #[tokio::test]
    async fn test_list() {
        let mut mock = MockMailTemplateDao::new();
        mock.expect_all().returning(|| Ok(Arc::from(vec![])));

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service.list().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn from_serde_json_error_maps_to_data_access() {
        let err = serde_json::from_str::<u32>("not a number").unwrap_err();
        let svc_err: MailTemplateError = err.into();
        assert!(
            matches!(&svc_err, MailTemplateError::DataAccess(msg) if msg.as_ref().contains("serialize failed")),
            "expected MailTemplateError::DataAccess with 'serialize failed'"
        );
    }

    // ── Phase 23 Plan 04 — HTML sanitize wiring (D-03 entry points 2 + 3) ──

    /// Phase 23 Plan 04 (HTML-05, D-03 entry point 2): `MailTemplateService::create`
    /// sanitizes `body_html` via `crate::sanitize::sanitize_html` before
    /// persisting to the DAO. `<script>` is stripped; safe tags survive.
    #[tokio::test]
    async fn create_sanitizes_body_html() {
        let mut mock = MockMailTemplateDao::new();
        mock.expect_find_by_name().returning(|_| Ok(None));

        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<Option<Arc<str>>>));
        let cap = captured.clone();
        mock.expect_create().returning(move |tpl| {
            *cap.lock().unwrap() = Some(tpl.body_html.clone());
            Ok(())
        });

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service
            .create(
                "sanitize-create",
                "Sub",
                "Body",
                Some("<p>Hi</p><script>alert(1)</script>".to_string()),
                "member",
            )
            .await;
        assert!(result.is_ok());

        let persisted = captured
            .lock()
            .unwrap()
            .clone()
            .expect("dao.create was invoked")
            .expect("body_html Some on persisted template");
        let s = persisted.as_ref();
        assert!(s.contains("<p>"), "safe <p> survives, got: {}", s);
        assert!(!s.contains("<script>"), "<script> stripped, got: {}", s);
    }

    /// Phase 23 Plan 04 (HTML-05, D-03 entry point 3): `MailTemplateService::update`
    /// sanitizes `body_html` via `crate::sanitize::sanitize_html` before
    /// persisting. Update takes full ownership of `body_html`.
    #[tokio::test]
    async fn update_sanitizes_body_html() {
        let existing_id = Uuid::new_v4();
        let existing_version = Uuid::new_v4();

        let mut mock = MockMailTemplateDao::new();
        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        mock.expect_find_by_id().returning(move |_| {
            Ok(Some(MailTemplate {
                id: existing_id,
                created,
                deleted: None,
                version: existing_version,
                name: Arc::from("sanitize-update"),
                subject: Arc::from("Sub"),
                body: Arc::from("Body"),
                body_html: None,
                template_type: Arc::from("member"),
            }))
        });
        mock.expect_find_by_name().returning(|_| Ok(None));

        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<Option<Arc<str>>>));
        let cap = captured.clone();
        mock.expect_update().returning(move |tpl| {
            *cap.lock().unwrap() = Some(tpl.body_html.clone());
            Ok(())
        });

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service
            .update(
                existing_id,
                "sanitize-update",
                "Sub",
                "Body",
                Some("<p>Hi</p><script>alert(1)</script>".to_string()),
                existing_version,
            )
            .await;
        assert!(result.is_ok());

        let persisted = captured
            .lock()
            .unwrap()
            .clone()
            .expect("dao.update was invoked")
            .expect("body_html Some on persisted template");
        let s = persisted.as_ref();
        assert!(s.contains("<p>"), "safe <p> survives, got: {}", s);
        assert!(!s.contains("<script>"), "<script> stripped, got: {}", s);
    }
}
