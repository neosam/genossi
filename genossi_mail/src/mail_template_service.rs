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
    async fn create(
        &self,
        name: &str,
        subject: &str,
        body: &str,
    ) -> Result<MailTemplate, MailTemplateError>;
    async fn update(
        &self,
        id: Uuid,
        name: &str,
        subject: &str,
        body: &str,
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
    ) -> Result<MailTemplate, MailTemplateError> {
        if let Some(_existing) = self.dao.find_by_name(name).await? {
            return Err(MailTemplateError::DuplicateName(Arc::from(name)));
        }

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        let template = MailTemplate {
            id: Uuid::new_v4(),
            created,
            deleted: None,
            version: Uuid::new_v4(),
            name: Arc::from(name),
            subject: Arc::from(subject),
            body: Arc::from(body),
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

        let updated = MailTemplate {
            id: existing.id,
            created: existing.created,
            deleted: None,
            version: Uuid::new_v4(),
            name: Arc::from(name),
            subject: Arc::from(subject),
            body: Arc::from(body),
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
        let result = service.create("Test", "Subject", "Body").await;
        assert!(result.is_ok());
        let tpl = result.unwrap();
        assert_eq!(tpl.name.as_ref(), "Test");
        assert_eq!(tpl.subject.as_ref(), "Subject");
        assert_eq!(tpl.body.as_ref(), "Body");
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
            }))
        });

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let result = service.create("Existing", "Sub", "Body").await;
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
            }))
        });

        let service = MailTemplateServiceImpl::new(Arc::new(mock));
        let wrong_version = Uuid::new_v4();
        let result = service
            .update(Uuid::new_v4(), "Test", "Sub", "Body", wrong_version)
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
}
