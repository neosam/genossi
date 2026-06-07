use async_trait::async_trait;
use genossi_config::service::ConfigService as ConfigServiceTrait;
use genossi_dao::application::{ApplicationDao, ApplicationStatus};
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::MemberActionDao;
use genossi_dao::TransactionDao;
use genossi_mail::service::MailService as MailServiceTrait;
use genossi_service::application::{
    Application, ApplicationService, ApplicationSubmission, ApplicationUpdate,
};
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
        };

        if let Err(e) = mail
            .create_job(
                &subject,
                &body,
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
