use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;
use time::PrimitiveDateTime;

use crate::DaoError;

#[derive(Debug, Clone)]
pub struct ProductEntity {
    pub id: Uuid,
    pub ean: Arc<str>,
    pub name: Arc<str>,
    pub short_name: Arc<str>,
    pub sales_unit: Arc<str>,
    pub requires_weighing: bool,
    pub price: i64, // Price in cents
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

#[async_trait]
pub trait ProductDao: Send + Sync {
    type Transaction: Send + Sync;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ProductEntity]>, DaoError>;
    
    async fn create(
        &self,
        entity: &ProductEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
    
    async fn update(
        &self,
        entity: &ProductEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
    
    // Default implementation that filters dump_all results
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[ProductEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ProductEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }
    
    // Default implementation that finds by EAN from dump_all results
    async fn find_by_ean(
        &self,
        ean: &str,
        tx: Self::Transaction,
    ) -> Result<Option<ProductEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.ean.as_ref() == ean)
            .cloned())
    }
    
    // Default implementation that finds by ID from dump_all results
    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ProductEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.id == id)
            .cloned())
    }
    
    // Default implementation that searches by name and EAN from dump_all results
    async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        tx: Self::Transaction,
    ) -> Result<Arc<[ProductEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let query_lower = query.to_lowercase();
        
        let mut matching_entities: Vec<ProductEntity> = all_entities
            .iter()
            .filter(|e| {
                e.deleted.is_none() && (
                    e.name.to_lowercase().contains(&query_lower) ||
                    e.ean.to_lowercase().contains(&query_lower) ||
                    e.short_name.to_lowercase().contains(&query_lower)
                )
            })
            .cloned()
            .collect();
            
        // Sort by relevance: exact matches first, then starts with, then contains
        matching_entities.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower || a.ean.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower || b.ean.to_lowercase() == query_lower;
            
            if a_exact && !b_exact {
                std::cmp::Ordering::Less
            } else if !a_exact && b_exact {
                std::cmp::Ordering::Greater
            } else {
                let a_starts = a.name.to_lowercase().starts_with(&query_lower) || a.ean.to_lowercase().starts_with(&query_lower);
                let b_starts = b.name.to_lowercase().starts_with(&query_lower) || b.ean.to_lowercase().starts_with(&query_lower);
                
                if a_starts && !b_starts {
                    std::cmp::Ordering::Less
                } else if !a_starts && b_starts {
                    std::cmp::Ordering::Greater
                } else {
                    a.name.cmp(&b.name)
                }
            }
        });
        
        if let Some(limit) = limit {
            matching_entities.truncate(limit);
        }
        
        Ok(matching_entities.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        Transaction {}
        impl Clone for Transaction {
            fn clone(&self) -> Self;
        }
        unsafe impl Send for Transaction {}
        unsafe impl Sync for Transaction {}
    }

    struct TestProductDao {
        entities: Vec<ProductEntity>,
    }

    #[async_trait]
    impl ProductDao for TestProductDao {
        type Transaction = MockTransaction;

        async fn dump_all(&self, _tx: Self::Transaction) -> Result<Arc<[ProductEntity]>, DaoError> {
            Ok(self.entities.clone().into())
        }

        async fn create(
            &self,
            _entity: &ProductEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }

        async fn update(
            &self,
            _entity: &ProductEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }
    }

    fn create_test_entity(id: Uuid, ean: &str, deleted: Option<PrimitiveDateTime>) -> ProductEntity {
        ProductEntity {
            id,
            ean: Arc::from(ean),
            name: Arc::from("Test Product"),
            short_name: Arc::from("Test"),
            sales_unit: Arc::from("St"),
            requires_weighing: false,
            price: 100,
            created: PrimitiveDateTime::MIN,
            deleted,
            version: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_all_filters_deleted_entities() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();
        
        let dao = TestProductDao {
            entities: vec![
                create_test_entity(id1, "123", None),
                create_test_entity(id2, "456", Some(PrimitiveDateTime::MIN)),
                create_test_entity(id3, "789", None),
            ],
        };
        
        let tx = MockTransaction::new();
        let result = dao.all(tx).await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|e| e.id == id1));
        assert!(result.iter().any(|e| e.id == id3));
        assert!(!result.iter().any(|e| e.id == id2));
    }

    #[tokio::test]
    async fn test_find_by_ean_returns_active_entity() {
        let id = Uuid::new_v4();
        let dao = TestProductDao {
            entities: vec![create_test_entity(id, "123456", None)],
        };
        
        let tx = MockTransaction::new();
        let result = dao.find_by_ean("123456", tx).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_find_by_ean_ignores_deleted_entity() {
        let id = Uuid::new_v4();
        let dao = TestProductDao {
            entities: vec![create_test_entity(id, "123456", Some(PrimitiveDateTime::MIN))],
        };
        
        let tx = MockTransaction::new();
        let result = dao.find_by_ean("123456", tx).await.unwrap();
        
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_id_returns_active_entity() {
        let id = Uuid::new_v4();
        let dao = TestProductDao {
            entities: vec![create_test_entity(id, "123456", None)],
        };
        
        let tx = MockTransaction::new();
        let result = dao.find_by_id(id, tx).await.unwrap();
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().ean.as_ref(), "123456");
    }

    #[tokio::test]
    async fn test_find_by_id_ignores_deleted_entity() {
        let id = Uuid::new_v4();
        let dao = TestProductDao {
            entities: vec![create_test_entity(id, "123456", Some(PrimitiveDateTime::MIN))],
        };
        
        let tx = MockTransaction::new();
        let result = dao.find_by_id(id, tx).await.unwrap();
        
        assert!(result.is_none());
    }
}