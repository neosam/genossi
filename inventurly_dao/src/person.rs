use std::sync::Arc;

use async_trait::async_trait;
use mockall::automock;
use uuid::Uuid;

use crate::DaoError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonEntity {
    pub id: Uuid,
    pub name: Arc<str>,
    pub age: i32,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait PersonDao {
    type Transaction: crate::Transaction;

    // Abstract methods - must be implemented by database-specific implementations
    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[PersonEntity]>, DaoError>;
    
    async fn create(
        &self,
        entity: &PersonEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
    
    async fn update(
        &self,
        entity: &PersonEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    // Default implementations - can be overridden if needed for optimization
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[PersonEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<PersonEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }
    
    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<PersonEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    
    // Test implementation that stores entities in memory
    struct TestPersonDao {
        entities: Mutex<Arc<[PersonEntity]>>,
    }
    
    impl TestPersonDao {
        fn new(entities: Vec<PersonEntity>) -> Self {
            Self {
                entities: Mutex::new(entities.into()),
            }
        }
    }
    
    #[async_trait]
    impl PersonDao for TestPersonDao {
        type Transaction = crate::MockTransaction;
        
        async fn dump_all(&self, _tx: Self::Transaction) -> Result<Arc<[PersonEntity]>, DaoError> {
            Ok(self.entities.lock().unwrap().clone())
        }
        
        async fn create(&self, _entity: &PersonEntity, _process: &str, _tx: Self::Transaction) -> Result<(), DaoError> {
            unimplemented!("Not needed for these tests")
        }
        
        async fn update(&self, _entity: &PersonEntity, _process: &str, _tx: Self::Transaction) -> Result<(), DaoError> {
            unimplemented!("Not needed for these tests")
        }
    }
    
    #[tokio::test]
    async fn test_all_filters_deleted_entities() {
        let entity1 = PersonEntity {
            id: Uuid::new_v4(),
            name: Arc::from("Active Person"),
            age: 30,
            created: time::PrimitiveDateTime::MIN,
            deleted: None,
            version: Uuid::new_v4(),
        };
        
        let entity2 = PersonEntity {
            id: Uuid::new_v4(),
            name: Arc::from("Deleted Person"),
            age: 25,
            created: time::PrimitiveDateTime::MIN,
            deleted: Some(time::PrimitiveDateTime::MIN),
            version: Uuid::new_v4(),
        };
        
        let entity3 = PersonEntity {
            id: Uuid::new_v4(),
            name: Arc::from("Another Active"),
            age: 35,
            created: time::PrimitiveDateTime::MIN,
            deleted: None,
            version: Uuid::new_v4(),
        };
        
        let dao = TestPersonDao::new(vec![entity1.clone(), entity2.clone(), entity3.clone()]);
        let tx = crate::MockTransaction::new();
        
        let result = dao.all(tx).await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|e| e.name.as_ref() == "Active Person"));
        assert!(result.iter().any(|e| e.name.as_ref() == "Another Active"));
        assert!(!result.iter().any(|e| e.name.as_ref() == "Deleted Person"));
    }
    
    #[tokio::test]
    async fn test_find_by_id_returns_active_entity() {
        let target_id = Uuid::new_v4();
        
        let entity1 = PersonEntity {
            id: target_id,
            name: Arc::from("Target Person"),
            age: 30,
            created: time::PrimitiveDateTime::MIN,
            deleted: None,
            version: Uuid::new_v4(),
        };
        
        let entity2 = PersonEntity {
            id: Uuid::new_v4(),
            name: Arc::from("Other Person"),
            age: 25,
            created: time::PrimitiveDateTime::MIN,
            deleted: None,
            version: Uuid::new_v4(),
        };
        
        let dao = TestPersonDao::new(vec![entity1.clone(), entity2.clone()]);
        let tx = crate::MockTransaction::new();
        
        let result = dao.find_by_id(target_id, tx).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().name.as_ref(), "Target Person");
    }
    
    #[tokio::test]
    async fn test_find_by_id_ignores_deleted_entity() {
        let target_id = Uuid::new_v4();
        
        let entity = PersonEntity {
            id: target_id,
            name: Arc::from("Deleted Person"),
            age: 30,
            created: time::PrimitiveDateTime::MIN,
            deleted: Some(time::PrimitiveDateTime::MIN),
            version: Uuid::new_v4(),
        };
        
        let dao = TestPersonDao::new(vec![entity.clone()]);
        let tx = crate::MockTransaction::new();
        
        let result = dao.find_by_id(target_id, tx).await.unwrap();
        
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_find_by_id_returns_none_for_nonexistent() {
        let target_id = Uuid::new_v4();
        let different_id = Uuid::new_v4();
        
        let entity = PersonEntity {
            id: different_id,
            name: Arc::from("Different Person"),
            age: 30,
            created: time::PrimitiveDateTime::MIN,
            deleted: None,
            version: Uuid::new_v4(),
        };
        
        let dao = TestPersonDao::new(vec![entity.clone()]);
        let tx = crate::MockTransaction::new();
        
        let result = dao.find_by_id(target_id, tx).await.unwrap();
        
        assert!(result.is_none());
    }
}