use std::sync::Arc;

use async_trait::async_trait;
use inventurly_dao::{
    person::PersonDao,
    TransactionDao,
};
use inventurly_service::{
    permission::{Authentication, ADMIN_PRIVILEGE, PermissionService},
    person::{Person, PersonService},
    uuid_service::UuidService,
    ServiceError, ValidationFailureItem,
};
use uuid::Uuid;

use crate::gen_service_impl;

gen_service_impl! {
    struct PersonServiceImpl: PersonService = PersonServiceDeps {
        PersonDao: PersonDao<Transaction = Self::Transaction> = person_dao,
        PermissionService: inventurly_service::permission::PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const PERSON_SERVICE_PROCESS: &str = "person-service";

#[async_trait]
impl<Deps: PersonServiceDeps> PersonService for PersonServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Person]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        
        let persons = self
            .person_dao
            .all(tx.clone())
            .await?
            .iter()
            .map(Person::from)
            .collect();
        
        self.transaction_dao.commit(tx).await?;
        Ok(persons)
    }

    async fn get(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Person, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        
        let person = self
            .person_dao
            .find_by_id(id, tx.clone())
            .await?
            .as_ref()
            .map(Person::from)
            .ok_or(ServiceError::EntityNotFound(id))?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(person)
    }

    async fn create(
        &self,
        item: &Person,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Person, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        let mut validation_errors = Vec::new();
        if item.name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("name"),
                message: Arc::from("Name cannot be empty"),
            });
        }
        if item.age < 0 || item.age > 150 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("age"),
                message: Arc::from("Age must be between 0 and 150"),
            });
        }
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        let now = time::OffsetDateTime::now_utc();
        let new_person = Person {
            id: self.uuid_service.new_v4().await,
            name: item.name.clone(),
            age: item.age,
            created: time::PrimitiveDateTime::new(now.date(), now.time()),
            deleted: None,
            version: self.uuid_service.new_v4().await,
        };

        self.person_dao
            .create(&(&new_person).into(), PERSON_SERVICE_PROCESS, tx.clone())
            .await?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(new_person)
    }

    async fn update(
        &self,
        item: &Person,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Person, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;

        // First check if the person exists
        let existing = self
            .person_dao
            .find_by_id(item.id, tx.clone())
            .await?;
        
        if existing.is_none() {
            return Err(ServiceError::EntityNotFound(item.id));
        }

        let mut validation_errors = Vec::new();
        if item.name.is_empty() {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("name"),
                message: Arc::from("Name cannot be empty"),
            });
        }
        if item.age < 0 || item.age > 150 {
            validation_errors.push(ValidationFailureItem {
                field: Arc::from("age"),
                message: Arc::from("Age must be between 0 and 150"),
            });
        }
        if !validation_errors.is_empty() {
            return Err(ServiceError::ValidationError(validation_errors));
        }

        self.person_dao
            .update(&item.into(), PERSON_SERVICE_PROCESS, tx.clone())
            .await?;
        
        let updated = self
            .person_dao
            .find_by_id(item.id, tx.clone())
            .await?
            .as_ref()
            .map(Person::from)
            .ok_or(ServiceError::EntityNotFound(item.id))?;
        
        self.transaction_dao.commit(tx).await?;
        Ok(updated)
    }

    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context)
            .await?;
        
        // Fetch the existing entity
        let existing = self
            .person_dao
            .find_by_id(id, tx.clone())
            .await?;
        
        match existing {
            Some(mut entity) => {
                // Set deleted timestamp
                let now = time::OffsetDateTime::now_utc();
                entity.deleted = Some(time::PrimitiveDateTime::new(now.date(), now.time()));
                
                // Update the entity with deleted timestamp
                self.person_dao
                    .update(&entity, PERSON_SERVICE_PROCESS, tx.clone())
                    .await?;
                
                self.transaction_dao.commit(tx).await?;
                Ok(())
            }
            None => Err(ServiceError::EntityNotFound(id))
        }
    }
}