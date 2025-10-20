use async_trait::async_trait;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::DaoError;

#[derive(Debug, Clone)]
pub struct ContainerEntity {
    pub id: Uuid,
    pub name: Arc<str>,
    pub weight_grams: i64,
    pub description: Arc<str>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

#[async_trait]
pub trait ContainerDao: Send + Sync {
    type Transaction: Send + Sync;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ContainerEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &ContainerEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &ContainerEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    // Default implementation that filters dump_all results
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[ContainerEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ContainerEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    // Default implementation that finds by ID from dump_all results
    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<ContainerEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.id == id)
            .cloned())
    }

    // Default implementation that finds by name from dump_all results
    async fn find_by_name(
        &self,
        name: &str,
        tx: Self::Transaction,
    ) -> Result<Option<ContainerEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.deleted.is_none() && e.name.as_ref() == name)
            .cloned())
    }

    // Default implementation that searches by name from dump_all results
    async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        tx: Self::Transaction,
    ) -> Result<Arc<[ContainerEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let query_lower = query.to_lowercase();

        let mut matching_entities: Vec<ContainerEntity> = all_entities
            .iter()
            .filter(|e| {
                e.deleted.is_none()
                    && (e.name.to_lowercase().contains(&query_lower)
                        || e.description.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        // Sort by relevance: exact matches first, then starts with, then contains
        matching_entities.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;

            if a_exact && !b_exact {
                std::cmp::Ordering::Less
            } else if !a_exact && b_exact {
                std::cmp::Ordering::Greater
            } else {
                let a_starts = a.name.to_lowercase().starts_with(&query_lower);
                let b_starts = b.name.to_lowercase().starts_with(&query_lower);

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

    struct TestContainerDao {
        entities: Vec<ContainerEntity>,
    }

    #[async_trait]
    impl ContainerDao for TestContainerDao {
        type Transaction = MockTransaction;

        async fn dump_all(
            &self,
            _tx: Self::Transaction,
        ) -> Result<Arc<[ContainerEntity]>, DaoError> {
            Ok(self.entities.clone().into())
        }

        async fn create(
            &self,
            _entity: &ContainerEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }

        async fn update(
            &self,
            _entity: &ContainerEntity,
            _process: &str,
            _tx: Self::Transaction,
        ) -> Result<(), DaoError> {
            Ok(())
        }
    }

    fn create_test_entity(
        id: Uuid,
        name: &str,
        deleted: Option<PrimitiveDateTime>,
    ) -> ContainerEntity {
        ContainerEntity {
            id,
            name: Arc::from(name),
            weight_grams: 1000,
            description: Arc::from("Test container"),
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

        let dao = TestContainerDao {
            entities: vec![
                create_test_entity(id1, "Container 1", None),
                create_test_entity(id2, "Container 2", Some(PrimitiveDateTime::MIN)),
                create_test_entity(id3, "Container 3", None),
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
    async fn test_find_by_name_returns_active_entity() {
        let id = Uuid::new_v4();
        let dao = TestContainerDao {
            entities: vec![create_test_entity(id, "Test Container", None)],
        };

        let tx = MockTransaction::new();
        let result = dao.find_by_name("Test Container", tx).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_find_by_name_ignores_deleted_entity() {
        let id = Uuid::new_v4();
        let dao = TestContainerDao {
            entities: vec![create_test_entity(
                id,
                "Test Container",
                Some(PrimitiveDateTime::MIN),
            )],
        };

        let tx = MockTransaction::new();
        let result = dao.find_by_name("Test Container", tx).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_id_returns_active_entity() {
        let id = Uuid::new_v4();
        let dao = TestContainerDao {
            entities: vec![create_test_entity(id, "Test Container", None)],
        };

        let tx = MockTransaction::new();
        let result = dao.find_by_id(id, tx).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name.as_ref(), "Test Container");
    }

    #[tokio::test]
    async fn test_find_by_id_ignores_deleted_entity() {
        let id = Uuid::new_v4();
        let dao = TestContainerDao {
            entities: vec![create_test_entity(
                id,
                "Test Container",
                Some(PrimitiveDateTime::MIN),
            )],
        };

        let tx = MockTransaction::new();
        let result = dao.find_by_id(id, tx).await.unwrap();

        assert!(result.is_none());
    }
}
