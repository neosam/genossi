use async_trait::async_trait;
use genossi_config::service::ConfigService as ConfigServiceTrait;
use genossi_dao::application::{ApplicationDao, ApplicationStatus};
use genossi_dao::application_document::ApplicationDocumentDao;
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::MemberActionDao;
use genossi_dao::member_document::MemberDocumentDao;
use genossi_dao::TransactionDao;
use genossi_mail::service::MailService as MailServiceTrait;
use genossi_service::application::{
    Application, ApplicationService, ApplicationSubmission, ApplicationUpdate,
};
use genossi_service::document_storage::{DocumentStorage, StorageError};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const APPLICATION_SERVICE_PROCESS: &str = "application-service";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";

gen_service_impl! {
    struct ApplicationServiceImpl: ApplicationService = ApplicationServiceDeps {
        ApplicationDao: ApplicationDao<Transaction = Self::Transaction> = application_dao,
        ApplicationDocumentDao: ApplicationDocumentDao<Transaction = Self::Transaction> = application_document_dao,
        MemberDocumentDao: MemberDocumentDao<Transaction = Self::Transaction> = member_document_dao,
        DocumentStorage: DocumentStorage = document_storage,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        ConfigService: genossi_config::service::ConfigService = config_service,
        MailService: genossi_mail::service::MailService = mail_service,
    }
}

impl<Deps: ApplicationServiceDeps> ApplicationServiceImpl<Deps> {
    async fn send_confirmation_mail(&self, app: &Application) {
        let email = match &app.email {
            Some(email) => email.to_string(),
            None => {
                tracing::error!("Cannot send confirmation mail: no email address");
                return;
            }
        };
        let config = self.config_service.clone();
        let mail = self.mail_service.clone();

        let share_value_cents = match config.get("share_value_cents").await {
            Ok(entry) => match entry.value.parse::<i64>() {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Failed to parse share_value_cents: {}", e);
                    return;
                }
            },
            Err(e) => {
                tracing::error!("Failed to read share_value_cents config: {:?}", e);
                return;
            }
        };

        let bank_iban = match config.get("bank_iban").await {
            Ok(entry) => entry.value.to_string(),
            Err(e) => {
                tracing::error!("Failed to read bank_iban config: {:?}", e);
                return;
            }
        };

        let bank_name = match config.get("bank_name").await {
            Ok(entry) => entry.value.to_string(),
            Err(e) => {
                tracing::error!("Failed to read bank_name config: {:?}", e);
                return;
            }
        };

        let bank_bic = config
            .get("bank_bic")
            .await
            .ok()
            .map(|e| e.value.to_string());

        let geno_name = match config.get("genossenschaft_name").await {
            Ok(entry) => entry.value.to_string(),
            Err(e) => {
                tracing::error!("Failed to read genossenschaft_name config: {:?}", e);
                return;
            }
        };

        let total_cents = share_value_cents * app.shares as i64;
        let euros = total_cents / 100;
        let cents = total_cents % 100;
        let amount_str = format!("{},{:02} €", euros, cents);

        let salutation_line = match &app.salutation {
            Some(s) => format!("Sehr geehrte/r {} {},", s.as_str(), app.last_name),
            None => format!("Sehr geehrte/r {} {},", app.first_name, app.last_name),
        };

        let bic_line = match &bank_bic {
            Some(bic) => format!("\nBIC: {}", bic),
            None => String::new(),
        };

        let body = format!(
            "{salutation_line}\n\n\
             vielen Dank für Ihre Beitrittserklärung zur {geno_name}.\n\n\
             Sie haben {shares} Geschäftsanteil(e) gezeichnet.\n\
             Bitte überweisen Sie den Betrag von {amount_str} auf folgendes Konto:\n\n\
             IBAN: {bank_iban}\n\
             Bank: {bank_name}{bic_line}\n\
             Verwendungszweck: Beitritt {first_name} {last_name}\n\n\
             Nach Zahlungseingang wird Ihr Beitritt bestätigt.\n\n\
             Mit freundlichen Grüßen\n\
             {geno_name}",
            shares = app.shares,
            first_name = app.first_name,
            last_name = app.last_name,
        );

        let subject = format!("Beitrittserklärung zur {}", geno_name);

        let recipient = genossi_mail::service::RecipientInput {
            address: email,
            member_id: None,
            // Phase 29 (APHIST-01): send_confirmation_mail geht an ein Mitglied, nicht
            // an einen Antragsteller — application_id bleibt None. Der echte
            // Application-Send kommt in Phase 31.
            application_id: None,
        };

        if let Err(e) = mail
            .create_job(
                &subject,
                &body,
                // Phase 23 Plan 04: application-confirmation mail is text-only.
                None,
                vec![recipient],
                vec![],
                vec![],
                // Phase 10 (Plan 10.03): application-confirmation mail is transactional,
                // not template/phase-driven — both stay None.
                None,
                None,
                // Quick 260603-cz6: confirmation mail is not a repayment-bulk send.
                false,
            )
            .await
        {
            tracing::error!("Failed to queue confirmation mail: {:?}", e);
        }
    }
}

#[async_trait]
impl<Deps: ApplicationServiceDeps> ApplicationService for ApplicationServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn submit(
        &self,
        submission: &ApplicationSubmission,
        send_mail: bool,
    ) -> Result<Application, ServiceError> {
        let mut validation_errors = Vec::new();

        if submission.first_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("first_name"),
                message: Arc::from("First name cannot be empty"),
            });
        }
        if submission.last_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("last_name"),
                message: Arc::from("Last name cannot be empty"),
            });
        }
        if submission.shares < 1 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("shares"),
                message: Arc::from("Shares must be at least 1"),
            });
        }
        if send_mail && submission.email.is_none() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("email"),
                message: Arc::from("Email is required when send_mail is true"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        let tx = self.transaction_dao.use_transaction(None).await?;

        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());

        let entity = genossi_dao::application::ApplicationEntity {
            id: self.uuid_service.new_v4().await,
            first_name: submission.first_name.clone(),
            last_name: submission.last_name.clone(),
            salutation: submission.salutation.clone(),
            title: submission.title.clone(),
            email: submission.email.clone(),
            street: submission.street.clone(),
            house_number: submission.house_number.clone(),
            postal_code: submission.postal_code.clone(),
            city: submission.city.clone(),
            shares: submission.shares,
            status: ApplicationStatus::Offen,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        crate::audited_create!(
            self,
            self.application_dao,
            &entity,
            APPLICATION_SERVICE_PROCESS,
            "PUBLIC",
            tx
        );

        self.transaction_dao.commit(tx).await?;

        let app = Application::from(&entity);

        if send_mail {
            self.send_confirmation_mail(&app).await;
        }

        Ok(app)
    }

    async fn list(
        &self,
        status_filter: Option<ApplicationStatus>,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[Application]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let all = self.application_dao.all(tx.clone()).await?;

        let filtered: Vec<Application> = all
            .iter()
            .filter(|e| status_filter.as_ref().is_none_or(|s| e.status == *s))
            .map(Application::from)
            .collect();

        self.transaction_dao.commit(tx).await?;
        Ok(filtered.into())
    }

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Application, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let entity = self
            .application_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        self.transaction_dao.commit(tx).await?;
        Ok(Application::from(&entity))
    }

    async fn confirm(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Application, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        // Phase 25 APDOC-02: fix CR-02 at this site. Permission check runs
        // BEFORE any user-attributable side effect so an unauthorized caller
        // cannot leak info via a partial-DAO trace.
        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
            .await?;

        let user_id = self
            .permission_service
            .current_user_id(context)
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        let mut entity = self
            .application_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        if entity.status != ApplicationStatus::Offen {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Application status is '{}', expected 'Offen'",
                entity.status.as_str()
            ))));
        }

        // Create member from application data
        let now = time::OffsetDateTime::now_utc();
        let created = time::PrimitiveDateTime::new(now.date(), now.time());
        let join_date = now.date();
        let member_number = self.member_dao.next_member_number(tx.clone()).await?;
        let member_id = self.uuid_service.new_v4().await;

        let member_entity = genossi_dao::member::MemberEntity {
            id: member_id,
            member_number,
            first_name: entity.first_name.clone(),
            last_name: entity.last_name.clone(),
            salutation: entity.salutation.clone(),
            title: entity.title.clone(),
            email: entity.email.clone(),
            company: None,
            comment: None,
            street: entity.street.clone(),
            house_number: entity.house_number.clone(),
            postal_code: entity.postal_code.clone(),
            city: entity.city.clone(),
            join_date,
            shares_at_joining: entity.shares,
            current_shares: entity.shares,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: genossi_dao::member::MemberStatus::Normal,
            // Quick 260607-mw9: account_holder column. Application-Konversion hat
            // keinen Kontoinhaber-Wert — Vorstand kann ihn manuell auf der
            // Member-Detail-Seite ergänzen.
            account_holder: None,
            // Quick 260625-e14: neue Mitglieder sind per Default postalisch erreichbar.
            postal_status: genossi_dao::member::PostalStatus::Erreichbar,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        crate::audited_create!(
            self,
            self.member_dao,
            &member_entity,
            APPLICATION_SERVICE_PROCESS,
            &user_id,
            tx
        );

        // Create Eintritt action
        let eintritt = genossi_dao::member_action::MemberActionEntity {
            id: self.uuid_service.new_v4().await,
            member_id,
            action_type: genossi_dao::member_action::ActionType::Eintritt,
            date: join_date,
            shares_change: 0,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };
        crate::audited_create!(
            self,
            self.member_action_dao,
            &eintritt,
            APPLICATION_SERVICE_PROCESS,
            &user_id,
            tx
        );

        // Create Aufstockung action
        let aufstockung = genossi_dao::member_action::MemberActionEntity {
            id: self.uuid_service.new_v4().await,
            member_id,
            action_type: genossi_dao::member_action::ActionType::Aufstockung,
            date: join_date,
            shares_change: entity.shares,
            transfer_member_id: None,
            effective_date: None,
            comment: None,
            created,
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };
        crate::audited_create!(
            self,
            self.member_action_dao,
            &aufstockung,
            APPLICATION_SERVICE_PROCESS,
            &user_id,
            tx
        );

        // Phase 25 (APDOC-03, APDOC-04): if an application document is attached,
        // Move-transfer it to an audited MemberDocument inside the same
        // transaction. Missing/corrupt file rolls back the whole cascade via
        // `?` propagation (APDOC-04 rollback guarantee). Old-file physical
        // delete runs AFTER commit (best-effort, warn-log only).
        let mut old_app_doc_path_for_cleanup: Option<String> = None;
        if let Some(app_doc) = self
            .application_document_dao
            .find_active_by_application_id(id, tx.clone())
            .await?
        {
            // 1. Load bytes. Missing file -> rollback via ? propagation.
            let bytes = self
                .document_storage
                .load(&app_doc.relative_path)
                .await
                .map_err(|e| match e {
                    StorageError::NotFound => ServiceError::InternalError(Arc::from(format!(
                        "Application document file missing on filesystem: {}",
                        app_doc.relative_path
                    ))),
                    other => ServiceError::InternalError(Arc::from(other.to_string())),
                })?;

            // 2. Compute member-doc relative path ("{uuid}.{ext}").
            let new_doc_id = self.uuid_service.new_v4().await;
            let extension = app_doc
                .file_name
                .rsplit('.')
                .next()
                .filter(|e| *e != app_doc.file_name.as_ref())
                .unwrap_or("bin");
            let new_relative_path = format!("{}.{}", new_doc_id, extension);

            // 3. Save under new path (fail -> rollback via ?).
            self.document_storage
                .save(&new_relative_path, &bytes)
                .await
                .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;

            // 4. Build DE-formatted description
            //    "Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)".
            let de_fmt = time::format_description::parse("[day].[month].[year]")
                .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;
            let join_date_str = join_date
                .format(&de_fmt)
                .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;
            let description_str = format!(
                "Original-Antrag (übernommen bei Bestätigung am {})",
                join_date_str
            );

            let member_doc_entity = genossi_dao::member_document::MemberDocumentEntity {
                id: new_doc_id,
                member_id,
                document_type: Arc::from("other"),
                description: Some(Arc::from(description_str.as_str())),
                file_name: app_doc.file_name.clone(),
                mime_type: app_doc.mime_type.clone(),
                relative_path: Arc::from(new_relative_path.as_str()),
                created,
                deleted: None,
                version: self.uuid_service.new_v4().await,
                template_id: None,
                mail_recipient_id: None,
                status: None,
            };

            // 5. audited_create! MemberDocument under APPLICATION_SERVICE_PROCESS.
            crate::audited_create!(
                self,
                self.member_document_dao,
                &member_doc_entity,
                APPLICATION_SERVICE_PROCESS,
                &user_id,
                tx
            );

            // 6. Soft-delete the application_document row (NOT audited).
            //    IMPORTANT: `entity.version` on ApplicationDocumentDao::update
            //    is the OLD version for the optimistic-lock WHERE clause; the
            //    DAO generates a fresh v4 internally as the NEW version. Passing
            //    a new UUID here (as an earlier draft did) makes every soft-
            //    delete blow up with `ConflictError("Version mismatch")` and
            //    cascades to a 409 out of confirm(). Discovered via e2e Plan
            //    25-05 Test E2E-1 (Rule 1 auto-fix during Wave 4).
            let old_relative_path = app_doc.relative_path.to_string();
            let now_dt = time::OffsetDateTime::now_utc();
            let soft_deleted_app_doc =
                genossi_dao::application_document::ApplicationDocumentEntity {
                    id: app_doc.id,
                    application_id: app_doc.application_id,
                    file_name: app_doc.file_name.clone(),
                    mime_type: app_doc.mime_type.clone(),
                    relative_path: app_doc.relative_path.clone(),
                    size: app_doc.size,
                    created: app_doc.created,
                    deleted: Some(time::PrimitiveDateTime::new(now_dt.date(), now_dt.time())),
                    version: app_doc.version,
                };
            self.application_document_dao
                .update(
                    &soft_deleted_app_doc,
                    APPLICATION_SERVICE_PROCESS,
                    tx.clone(),
                )
                .await?;

            // 7. Remember old path for best-effort delete AFTER commit.
            old_app_doc_path_for_cleanup = Some(old_relative_path);
        }

        // Update application status
        entity.status = ApplicationStatus::Bestaetigt;
        crate::audited_update!(
            self,
            self.application_dao,
            id,
            &entity,
            APPLICATION_SERVICE_PROCESS,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;

        // Phase 25 APDOC-04 best-effort: delete old application-doc file AFTER
        // the transaction has committed. A failure here does NOT roll back —
        // the Member is already activated and the DB row is soft-deleted. The
        // orphan file (if any) sits at a UUID path unreachable via API.
        if let Some(old_path) = old_app_doc_path_for_cleanup {
            if let Err(e) = self.document_storage.delete(&old_path).await {
                tracing::warn!(
                    old_path = %old_path,
                    error = ?e,
                    "Failed to delete old application document file after confirm (best-effort)",
                );
            }
        }

        Ok(Application::from(&entity))
    }

    async fn reject(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Application, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let mut entity = self
            .application_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        if entity.status != ApplicationStatus::Offen {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Application status is '{}', expected 'Offen'",
                entity.status.as_str()
            ))));
        }

        entity.status = ApplicationStatus::Abgelehnt;
        crate::audited_update!(
            self,
            self.application_dao,
            id,
            &entity,
            APPLICATION_SERVICE_PROCESS,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(Application::from(&entity))
    }

    async fn update_application(
        &self,
        id: Uuid,
        update: &ApplicationUpdate,
        context: Authentication<Self::Context>,
    ) -> Result<Application, ServiceError> {
        let mut validation_errors = Vec::new();

        if update.first_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("first_name"),
                message: Arc::from("First name cannot be empty"),
            });
        }
        if update.last_name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("last_name"),
                message: Arc::from("Last name cannot be empty"),
            });
        }
        if update.shares < 1 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("shares"),
                message: Arc::from("Shares must be at least 1"),
            });
        }

        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        let tx = self.transaction_dao.use_transaction(None).await?;

        let user_id = self
            .permission_service
            .current_user_id(context.clone())
            .await?
            .unwrap_or_else(|| "SYSTEM".to_string());

        self.permission_service
            .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
            .await?;

        let mut entity = self
            .application_dao
            .find_by_id(id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(id))?;

        if entity.version != update.version {
            return Err(ServiceError::Conflict(Arc::from(
                "Version mismatch: the application has been modified by another user",
            )));
        }

        entity.first_name = update.first_name.clone();
        entity.last_name = update.last_name.clone();
        entity.salutation = update.salutation.clone();
        entity.title = update.title.clone();
        entity.email = update.email.clone();
        entity.street = update.street.clone();
        entity.house_number = update.house_number.clone();
        entity.postal_code = update.postal_code.clone();
        entity.city = update.city.clone();
        entity.shares = update.shares;

        crate::audited_update!(
            self,
            self.application_dao,
            id,
            &entity,
            APPLICATION_SERVICE_PROCESS,
            &user_id,
            tx
        );

        self.transaction_dao.commit(tx).await?;
        Ok(Application::from(&entity))
    }
}

#[cfg(test)]
mod tests {
    //! Phase 25 (APDOC-02/03/04) tests for `ApplicationServiceImpl::confirm()`:
    //!
    //! - **Test A** — CR-02 fix regression guard: unauthorised confirm() must NOT
    //!   touch DAOs / storage / capture user id.
    //! - **Test B** — Happy carryover with document: MemberDocument is created via
    //!   audited path; application_document row is soft-deleted; old storage
    //!   file is best-effort deleted AFTER commit.
    //! - **Test C** — No document → skip carryover; storage.load/save/delete are
    //!   never called; MemberDocument DAO create is never called.
    //! - **Test D** — Missing file → full rollback: `storage.load` returns
    //!   NotFound; confirm() returns InternalError; the tx.commit call is
    //!   NEVER made (rollback guarantee, APDOC-04).
    //!
    //! Mock setup: `MockContext` (from `genossi_service::permission`) is the
    //! Context type; all DAOs and storage are hand-rolled mocks via
    //! `mockall::mock!` because `#[automock(...)]` on the traits uses `()`
    //! for the Context, which is incompatible with our Deps assoc-type.

    use super::*;
    use async_trait::async_trait;
    use genossi_config::service::MockConfigService;
    use genossi_dao::application::{ApplicationDao, ApplicationEntity};
    use genossi_dao::application_document::{ApplicationDocumentDao, ApplicationDocumentEntity};
    use genossi_dao::audit_log::{AuditLogDao, AuditLogEntry, AuditQueryFilter};
    use genossi_dao::member::{MemberDao, MemberEntity};
    use genossi_dao::member_action::{MemberActionDao, MemberActionEntity};
    use genossi_dao::member_document::{MemberDocumentDao, MemberDocumentEntity};
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::mock;
    use std::sync::atomic::{AtomicBool, Ordering};

    // ---- Test transaction type (no-op) ----

    #[derive(Clone, Debug)]
    pub struct TestTx;

    #[async_trait]
    impl Transaction for TestTx {
        async fn begin(&mut self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn commit(self) -> Result<(), DaoError> {
            Ok(())
        }
        async fn rollback(self) -> Result<(), DaoError> {
            Ok(())
        }
    }

    // ---- Mock DAOs / services ----

    mock! {
        pub TxDao {}
        #[async_trait]
        impl TransactionDao for TxDao {
            type Transaction = TestTx;
            async fn transaction(&self) -> Result<TestTx, DaoError>;
            async fn use_transaction(&self, tx: Option<TestTx>) -> Result<TestTx, DaoError>;
            async fn commit(&self, tx: TestTx) -> Result<(), DaoError>;
        }
    }

    mock! {
        pub AppDao {}
        #[async_trait]
        impl ApplicationDao for AppDao {
            type Transaction = TestTx;
            async fn dump_all(&self, tx: TestTx) -> Result<Arc<[ApplicationEntity]>, DaoError>;
            async fn create(&self, entity: &ApplicationEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn update(&self, entity: &ApplicationEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTx) -> Result<Arc<[ApplicationEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTx) -> Result<Option<ApplicationEntity>, DaoError>;
        }
    }

    mock! {
        pub AppDocDao {}
        #[async_trait]
        impl ApplicationDocumentDao for AppDocDao {
            type Transaction = TestTx;
            async fn dump_all(&self, tx: TestTx) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError>;
            async fn create(&self, entity: &ApplicationDocumentEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn update(&self, entity: &ApplicationDocumentEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTx) -> Result<Arc<[ApplicationDocumentEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTx) -> Result<Option<ApplicationDocumentEntity>, DaoError>;
            async fn find_active_by_application_id(&self, application_id: Uuid, tx: TestTx) -> Result<Option<ApplicationDocumentEntity>, DaoError>;
        }
    }

    mock! {
        pub MemDao {}
        #[async_trait]
        impl MemberDao for MemDao {
            type Transaction = TestTx;
            async fn dump_all(&self, tx: TestTx) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn create(&self, entity: &MemberEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTx) -> Result<Arc<[MemberEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTx) -> Result<Option<MemberEntity>, DaoError>;
            async fn update_migrated(&self, id: Uuid, migrated: bool, tx: TestTx) -> Result<(), DaoError>;
            async fn update_dates(&self, id: Uuid, join_date: time::Date, exit_date: Option<time::Date>, tx: TestTx) -> Result<(), DaoError>;
            async fn find_by_member_number(&self, member_number: i64, tx: TestTx) -> Result<Option<MemberEntity>, DaoError>;
            async fn next_member_number(&self, tx: TestTx) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub MemActionDao {}
        #[async_trait]
        impl MemberActionDao for MemActionDao {
            type Transaction = TestTx;
            async fn dump_all(&self, tx: TestTx) -> Result<Arc<[MemberActionEntity]>, DaoError>;
            async fn create(&self, entity: &MemberActionEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberActionEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTx) -> Result<Arc<[MemberActionEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTx) -> Result<Option<MemberActionEntity>, DaoError>;
            async fn find_by_member_id(&self, member_id: Uuid, tx: TestTx) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        }
    }

    mock! {
        pub MemDocDao {}
        #[async_trait]
        impl MemberDocumentDao for MemDocDao {
            type Transaction = TestTx;
            async fn dump_all(&self, tx: TestTx) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;
            async fn create(&self, entity: &MemberDocumentEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn update(&self, entity: &MemberDocumentEntity, process: &str, tx: TestTx) -> Result<(), DaoError>;
            async fn all(&self, tx: TestTx) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;
            async fn find_by_id(&self, id: Uuid, tx: TestTx) -> Result<Option<MemberDocumentEntity>, DaoError>;
            async fn find_by_member_id(&self, member_id: Uuid, tx: TestTx) -> Result<Arc<[MemberDocumentEntity]>, DaoError>;
            async fn count_by_type(&self, document_type: &str, tx: TestTx) -> Result<std::collections::HashMap<Uuid, i64>, DaoError>;
        }
    }

    mock! {
        pub AudLogDao {}
        #[async_trait]
        impl AuditLogDao for AudLogDao {
            type Transaction = TestTx;
            async fn create_entries(&self, entries: &[AuditLogEntry], tx: TestTx) -> Result<(), DaoError>;
            async fn get_latest_hash(&self, tx: TestTx) -> Result<Option<String>, DaoError>;
            async fn get_by_entity(&self, entity_type: &str, entity_id: Uuid, tx: TestTx) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn get_all_ordered(&self, tx: TestTx) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn query(&self, filter: AuditQueryFilter, limit: i64, offset: i64, tx: TestTx) -> Result<Arc<[AuditLogEntry]>, DaoError>;
            async fn count(&self, filter: AuditQueryFilter, tx: TestTx) -> Result<i64, DaoError>;
        }
    }

    mock! {
        pub Storage {}
        #[async_trait]
        impl DocumentStorage for Storage {
            async fn save(&self, relative_path: &str, data: &[u8]) -> Result<(), StorageError>;
            async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>;
            async fn delete(&self, relative_path: &str) -> Result<(), StorageError>;
        }
    }

    mock! {
        pub PermSvc {}
        #[async_trait]
        impl PermissionService for PermSvc {
            type Context = MockContext;
            async fn check_permission(&self, privilege: &str, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn current_user_id(&self, context: Authentication<MockContext>) -> Result<Option<String>, ServiceError>;
            async fn get_all_users(&self, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::UserResponseTO]>, ServiceError>;
            async fn create_user(&self, user: genossi_service::auth_types::UserTO, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn delete_user(&self, username: String, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_all_roles(&self, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn create_role(&self, role: genossi_service::auth_types::RoleTO, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn delete_role(&self, role_name: String, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_all_privileges(&self, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn create_privilege(&self, privilege: genossi_service::auth_types::PrivilegeTO, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn delete_privilege(&self, privilege_name: String, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn assign_user_role(&self, user_role: genossi_service::auth_types::UserRole, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn remove_user_role(&self, user_role: genossi_service::auth_types::UserRole, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_user_roles(&self, username: String, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::RoleResponseTO]>, ServiceError>;
            async fn assign_role_privilege(&self, role_privilege: genossi_service::auth_types::RolePrivilege, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn remove_role_privilege(&self, role_privilege: genossi_service::auth_types::RolePrivilege, context: Authentication<MockContext>) -> Result<(), ServiceError>;
            async fn get_role_privileges(&self, role_name: String, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn get_user_privileges(&self, username: String, context: Authentication<MockContext>) -> Result<Arc<[genossi_service::auth_types::PrivilegeResponseTO]>, ServiceError>;
            async fn has_claims(&self, context: &MockContext) -> Result<bool, ServiceError>;
        }
    }

    #[derive(Clone)]
    struct RngUuid;
    #[async_trait]
    impl UuidService for RngUuid {
        async fn new_v4(&self) -> Uuid {
            Uuid::new_v4()
        }
    }

    // Config / Mail services — never invoked in the confirm() flow. Reuse the
    // automock-generated mocks (`MockConfigService`, `MockMailService`) so we
    // don't have to hand-maintain trait signatures.

    // ---- Deps binding for tests ----

    struct TestDeps;
    impl ApplicationServiceDeps for TestDeps {
        type Context = MockContext;
        type Transaction = TestTx;
        type ApplicationDao = MockAppDao;
        type ApplicationDocumentDao = MockAppDocDao;
        type MemberDocumentDao = MockMemDocDao;
        type DocumentStorage = MockStorage;
        type AuditLogDao = MockAudLogDao;
        type MemberDao = MockMemDao;
        type MemberActionDao = MockMemActionDao;
        type PermissionService = MockPermSvc;
        type UuidService = RngUuid;
        type TransactionDao = MockTxDao;
        type ConfigService = MockConfigService;
        type MailService = genossi_mail::service::MockMailService;
    }

    // ---- Helpers ----

    fn make_dt() -> time::PrimitiveDateTime {
        let d = time::Date::from_calendar_date(2026, time::Month::July, 3).unwrap();
        time::PrimitiveDateTime::new(d, time::Time::MIDNIGHT)
    }

    fn app_entity_offen(id: Uuid) -> ApplicationEntity {
        ApplicationEntity {
            id,
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mustermann"),
            salutation: None,
            title: None,
            email: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            shares: 5,
            status: ApplicationStatus::Offen,
            created: make_dt(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn app_doc(app_id: Uuid) -> ApplicationDocumentEntity {
        ApplicationDocumentEntity {
            id: Uuid::new_v4(),
            application_id: app_id,
            file_name: Arc::from("original_antrag.pdf"),
            mime_type: Arc::from("application/pdf"),
            relative_path: Arc::from("applications/foo/original.pdf"),
            size: 4096,
            created: make_dt(),
            deleted: None,
            version: Uuid::new_v4(),
        }
    }

    fn permission_ok() -> MockPermSvc {
        let mut p = MockPermSvc::new();
        p.expect_check_permission().returning(|_, _| Ok(()));
        p.expect_current_user_id()
            .returning(|_| Ok(Some("admin".to_string())));
        p
    }

    fn audit_log_ok() -> MockAudLogDao {
        let mut a = MockAudLogDao::new();
        a.expect_get_latest_hash().returning(|_| Ok(None));
        a.expect_create_entries().returning(|_, _| Ok(()));
        a
    }

    fn build_service(
        app_dao: MockAppDao,
        app_doc_dao: MockAppDocDao,
        member_doc_dao: MockMemDocDao,
        storage: MockStorage,
        audit: MockAudLogDao,
        member_dao: MockMemDao,
        action_dao: MockMemActionDao,
        perm: MockPermSvc,
        tx_dao: MockTxDao,
    ) -> ApplicationServiceImpl<TestDeps> {
        ApplicationServiceImpl {
            application_dao: Arc::new(app_dao),
            application_document_dao: Arc::new(app_doc_dao),
            member_document_dao: Arc::new(member_doc_dao),
            document_storage: Arc::new(storage),
            audit_log_dao: Arc::new(audit),
            member_dao: Arc::new(member_dao),
            member_action_dao: Arc::new(action_dao),
            permission_service: Arc::new(perm),
            uuid_service: Arc::new(RngUuid),
            transaction_dao: Arc::new(tx_dao),
            config_service: Arc::new(MockConfigService::new()),
            mail_service: Arc::new(genossi_mail::service::MockMailService::new()),
        }
    }

    /// **Test A — CR-02 fix regression guard (APDOC-02).**
    ///
    /// `check_permission` returns `PermissionDenied`; `current_user_id` MUST
    /// NOT be called; NO DAO / storage side effects allowed.
    #[tokio::test]
    async fn test_confirm_cr02_permission_denied_has_no_side_effects() {
        let app_id = Uuid::new_v4();

        let mut app_dao = MockAppDao::new();
        app_dao.expect_find_by_id().times(0);
        app_dao.expect_update().times(0);

        let mut app_doc_dao = MockAppDocDao::new();
        app_doc_dao.expect_find_active_by_application_id().times(0);
        app_doc_dao.expect_update().times(0);

        let mut member_doc_dao = MockMemDocDao::new();
        member_doc_dao.expect_create().times(0);

        let mut member_dao = MockMemDao::new();
        member_dao.expect_next_member_number().times(0);
        member_dao.expect_create().times(0);

        let mut action_dao = MockMemActionDao::new();
        action_dao.expect_create().times(0);

        let mut storage = MockStorage::new();
        storage.expect_load().times(0);
        storage.expect_save().times(0);
        storage.expect_delete().times(0);

        let mut perm = MockPermSvc::new();
        perm.expect_check_permission()
            .times(1)
            .returning(|_, _| Err(ServiceError::PermissionDenied));
        perm.expect_current_user_id().times(0);

        let mut tx_dao = MockTxDao::new();
        tx_dao.expect_use_transaction().returning(|_| Ok(TestTx));
        tx_dao.expect_commit().times(0);

        let svc = build_service(
            app_dao,
            app_doc_dao,
            member_doc_dao,
            storage,
            audit_log_ok(),
            member_dao,
            action_dao,
            perm,
            tx_dao,
        );

        let err = svc
            .confirm(app_id, Authentication::Context(MockContext))
            .await
            .expect_err("unauthorised confirm must fail");
        assert!(matches!(err, ServiceError::PermissionDenied));
    }

    /// **Test B — Happy carryover with document (APDOC-03).**
    ///
    /// Confirm with an attached application_document → the storage load/save
    /// sequence runs, MemberDocument DAO create is called, application_document
    /// row is soft-deleted, tx.commit is called, and the old file is deleted
    /// AFTER commit (best-effort).
    #[tokio::test]
    async fn test_confirm_with_document_creates_audited_member_doc_and_soft_deletes() {
        let app_id = Uuid::new_v4();
        let existing_doc = app_doc(app_id);
        let old_path = existing_doc.relative_path.clone();

        let mut app_dao = MockAppDao::new();
        {
            let app = app_entity_offen(app_id);
            app_dao
                .expect_find_by_id()
                .returning(move |_, _| Ok(Some(app.clone())));
        }
        app_dao
            .expect_update()
            .times(1)
            .withf(|entity: &ApplicationEntity, _p: &str, _tx: &TestTx| {
                entity.status == ApplicationStatus::Bestaetigt
            })
            .returning(|_, _, _| Ok(()));
        // audited_update loads the OLD entity via find_by_id first; already
        // covered by the general returning() above (finds the same offen row).
        // Add the second lookup — mockall allows multiple invocations on the
        // returning() setup automatically.

        let mut app_doc_dao = MockAppDocDao::new();
        {
            let doc = existing_doc.clone();
            app_doc_dao
                .expect_find_active_by_application_id()
                .returning(move |_, _| Ok(Some(doc.clone())));
        }
        // Soft-delete update on the app_doc row.
        app_doc_dao
            .expect_update()
            .times(1)
            .withf(
                |entity: &ApplicationDocumentEntity, _p: &str, _tx: &TestTx| {
                    entity.deleted.is_some()
                },
            )
            .returning(|_, _, _| Ok(()));

        let mut member_doc_dao = MockMemDocDao::new();
        // MemberDocument audited create → 1 call with description containing
        // the DE prefix.
        let saw_de_desc = Arc::new(AtomicBool::new(false));
        {
            let flag = saw_de_desc.clone();
            member_doc_dao
                .expect_create()
                .times(1)
                .withf(
                    move |entity: &MemberDocumentEntity, _p: &str, _tx: &TestTx| {
                        let ok = entity
                            .description
                            .as_ref()
                            .map(|d| {
                                d.starts_with("Original-Antrag (übernommen bei Bestätigung am ")
                            })
                            .unwrap_or(false)
                            && entity.document_type.as_ref() == "other";
                        if ok {
                            flag.store(true, Ordering::SeqCst);
                        }
                        ok
                    },
                )
                .returning(|_, _, _| Ok(()));
        }

        let mut member_dao = MockMemDao::new();
        member_dao
            .expect_next_member_number()
            .returning(|_| Ok(1001));
        member_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut action_dao = MockMemActionDao::new();
        action_dao
            .expect_create()
            .times(2)
            .returning(|_, _, _| Ok(()));

        let mut storage = MockStorage::new();
        storage
            .expect_load()
            .times(1)
            .returning(|_| Ok(b"pdfbytes".to_vec()));
        storage.expect_save().times(1).returning(|_, _| Ok(()));
        // Best-effort delete of old path AFTER commit.
        {
            let expected = old_path.to_string();
            storage
                .expect_delete()
                .times(1)
                .withf(move |p| p == expected)
                .returning(|_| Ok(()));
        }

        let mut tx_dao = MockTxDao::new();
        tx_dao.expect_use_transaction().returning(|_| Ok(TestTx));
        tx_dao.expect_commit().times(1).returning(|_| Ok(()));

        let svc = build_service(
            app_dao,
            app_doc_dao,
            member_doc_dao,
            storage,
            audit_log_ok(),
            member_dao,
            action_dao,
            permission_ok(),
            tx_dao,
        );

        let confirmed = svc
            .confirm(app_id, Authentication::Context(MockContext))
            .await
            .expect("confirm with attached document must succeed");
        assert_eq!(confirmed.status, ApplicationStatus::Bestaetigt);
        assert!(
            saw_de_desc.load(Ordering::SeqCst),
            "MemberDocument description must be DE-formatted 'Original-Antrag ...'"
        );
    }

    /// **Test C — No document → skip carryover.**
    ///
    /// `find_active_by_application_id` returns `None`; confirm() still
    /// succeeds; storage.load/save/delete are never called; MemberDocument
    /// DAO create is never called.
    #[tokio::test]
    async fn test_confirm_without_document_skips_carryover() {
        let app_id = Uuid::new_v4();

        let mut app_dao = MockAppDao::new();
        {
            let app = app_entity_offen(app_id);
            app_dao
                .expect_find_by_id()
                .returning(move |_, _| Ok(Some(app.clone())));
        }
        app_dao.expect_update().times(1).returning(|_, _, _| Ok(()));

        let mut app_doc_dao = MockAppDocDao::new();
        app_doc_dao
            .expect_find_active_by_application_id()
            .returning(|_, _| Ok(None));
        app_doc_dao.expect_update().times(0);

        let mut member_doc_dao = MockMemDocDao::new();
        member_doc_dao.expect_create().times(0);

        let mut member_dao = MockMemDao::new();
        member_dao
            .expect_next_member_number()
            .returning(|_| Ok(1002));
        member_dao
            .expect_create()
            .times(1)
            .returning(|_, _, _| Ok(()));

        let mut action_dao = MockMemActionDao::new();
        action_dao
            .expect_create()
            .times(2)
            .returning(|_, _, _| Ok(()));

        let mut storage = MockStorage::new();
        storage.expect_load().times(0);
        storage.expect_save().times(0);
        storage.expect_delete().times(0);

        let mut tx_dao = MockTxDao::new();
        tx_dao.expect_use_transaction().returning(|_| Ok(TestTx));
        tx_dao.expect_commit().times(1).returning(|_| Ok(()));

        let svc = build_service(
            app_dao,
            app_doc_dao,
            member_doc_dao,
            storage,
            audit_log_ok(),
            member_dao,
            action_dao,
            permission_ok(),
            tx_dao,
        );

        let confirmed = svc
            .confirm(app_id, Authentication::Context(MockContext))
            .await
            .expect("confirm without doc must succeed");
        assert_eq!(confirmed.status, ApplicationStatus::Bestaetigt);
    }

    /// **Test D — Missing file → full rollback (APDOC-04).**
    ///
    /// `storage.load` returns `NotFound`; confirm() must return
    /// `InternalError` and MUST NOT call `tx.commit`. The Member DAO create
    /// may have run before the load attempt — but the transaction abort
    /// unwinds it.
    #[tokio::test]
    async fn test_confirm_missing_file_rolls_back_full_transaction() {
        let app_id = Uuid::new_v4();
        let existing_doc = app_doc(app_id);

        let mut app_dao = MockAppDao::new();
        {
            let app = app_entity_offen(app_id);
            app_dao
                .expect_find_by_id()
                .returning(move |_, _| Ok(Some(app.clone())));
        }
        // The final application audited_update MUST NOT be reached in
        // rollback path.
        app_dao.expect_update().times(0);

        let mut app_doc_dao = MockAppDocDao::new();
        {
            let doc = existing_doc.clone();
            app_doc_dao
                .expect_find_active_by_application_id()
                .returning(move |_, _| Ok(Some(doc.clone())));
        }
        // No soft-delete update — we fail before that step.
        app_doc_dao.expect_update().times(0);

        let mut member_doc_dao = MockMemDocDao::new();
        // MemberDocument create MUST NOT be called — storage load fails first.
        member_doc_dao.expect_create().times(0);

        let mut member_dao = MockMemDao::new();
        member_dao
            .expect_next_member_number()
            .returning(|_| Ok(1003));
        // MemberEntity create is called BEFORE the carryover branch; that's
        // fine — the rollback will undo it. We accept any number of calls
        // here; the semantic invariant is that tx.commit is never reached.
        member_dao.expect_create().returning(|_, _, _| Ok(()));

        let mut action_dao = MockMemActionDao::new();
        action_dao.expect_create().returning(|_, _, _| Ok(()));

        let mut storage = MockStorage::new();
        // The load FAILS — this is the APDOC-04 rollback trigger.
        storage
            .expect_load()
            .times(1)
            .returning(|_| Err(StorageError::NotFound));
        storage.expect_save().times(0);
        storage.expect_delete().times(0);

        let mut tx_dao = MockTxDao::new();
        tx_dao.expect_use_transaction().returning(|_| Ok(TestTx));
        // CRITICAL invariant: commit is NEVER called on the rollback path.
        tx_dao.expect_commit().times(0);

        let svc = build_service(
            app_dao,
            app_doc_dao,
            member_doc_dao,
            storage,
            audit_log_ok(),
            member_dao,
            action_dao,
            permission_ok(),
            tx_dao,
        );

        let err = svc
            .confirm(app_id, Authentication::Context(MockContext))
            .await
            .expect_err("missing file must trigger rollback");
        match err {
            ServiceError::InternalError(msg) => {
                assert!(
                    msg.contains("missing on filesystem"),
                    "error must mention filesystem corruption, got: {}",
                    msg
                );
            }
            other => panic!("expected InternalError, got {:?}", other),
        }
    }
}
