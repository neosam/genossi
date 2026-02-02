use async_trait::async_trait;
use std::sync::Arc;

use inventurly_dao::TransactionDao;
use inventurly_service::{
    duplicate_detection::{
        DuplicateDetectionConfig, DuplicateDetectionResult, DuplicateDetectionService,
        DuplicateMatch, MatchConfidence, SimilarityCalculator,
    },
    permission::{Authentication, PermissionService},
    product::Product,
    product::ProductService,
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct DuplicateDetectionServiceImpl: DuplicateDetectionService = DuplicateDetectionServiceDeps {
        ProductService: ProductService<Context = Self::Context, Transaction = Self::Transaction> = product_service,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

#[async_trait]
impl<Deps: DuplicateDetectionServiceDeps> DuplicateDetectionService
    for DuplicateDetectionServiceImpl<Deps>
{
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn find_duplicates(
        &self,
        product: &Product,
        config: Option<DuplicateDetectionConfig>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<DuplicateDetectionResult, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        let config = config.unwrap_or_default();

        // Get all products except the one we're checking
        let all_products = self
            .product_service
            .get_all(context, Some(tx.clone()))
            .await?;
        let mut matches = Vec::new();

        for other_product in all_products.iter() {
            // Skip comparing with itself
            if other_product.id == product.id {
                continue;
            }

            let (similarity_score, algorithm_scores) =
                SimilarityCalculator::calculate_similarity(product, other_product, &config);

            if similarity_score >= config.similarity_threshold {
                let confidence = MatchConfidence::from_score(similarity_score);
                matches.push(DuplicateMatch {
                    product: other_product.clone(),
                    similarity_score,
                    algorithm_scores,
                    confidence,
                });
            }
        }

        // Sort matches by similarity score (highest first)
        matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        self.transaction_dao.commit(tx).await?;

        Ok(DuplicateDetectionResult {
            checked_product: product.clone(),
            matches,
            config,
        })
    }

    async fn find_all_duplicates(
        &self,
        config: Option<DuplicateDetectionConfig>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Vec<DuplicateDetectionResult>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        let config = config.unwrap_or_default();

        let all_products = self
            .product_service
            .get_all(context.clone(), Some(tx.clone()))
            .await?;
        let mut results = Vec::new();

        for product in all_products.iter() {
            let result = self
                .find_duplicates(
                    product,
                    Some(config.clone()),
                    context.clone(),
                    Some(tx.clone()),
                )
                .await?;

            // Only include products that have potential duplicates
            if !result.matches.is_empty() {
                results.push(result);
            }
        }

        // Sort results by highest similarity score found in each result's matches (highest first)
        results.sort_by(|a, b| {
            let max_score_a = a.matches.iter()
                .map(|m| m.similarity_score)
                .fold(0.0, f64::max);
            let max_score_b = b.matches.iter()
                .map(|m| m.similarity_score)
                .fold(0.0, f64::max);
            max_score_b.partial_cmp(&max_score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        self.transaction_dao.commit(tx).await?;
        Ok(results)
    }

    async fn check_potential_duplicate(
        &self,
        name: &str,
        sales_unit: &str,
        requires_weighing: bool,
        config: Option<DuplicateDetectionConfig>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Vec<DuplicateMatch>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        let config = config.unwrap_or_default();

        // Create a temporary product for comparison
        let temp_product = Product {
            id: uuid::Uuid::new_v4(), // Temporary ID
            ean: Arc::from("temp"),
            name: Arc::from(name),
            short_name: Arc::from(name),
            sales_unit: Arc::from(sales_unit),
            requires_weighing,
            price: inventurly_service::product::Price::from_cents(0),
            deposit: inventurly_service::product::Price::from_cents(0),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            deleted: None,
            version: uuid::Uuid::new_v4(),
        };

        let all_products = self
            .product_service
            .get_all(context, Some(tx.clone()))
            .await?;
        let mut matches = Vec::new();

        for existing_product in all_products.iter() {
            let (similarity_score, algorithm_scores) = SimilarityCalculator::calculate_similarity(
                &temp_product,
                existing_product,
                &config,
            );

            if similarity_score >= config.similarity_threshold {
                let confidence = MatchConfidence::from_score(similarity_score);
                matches.push(DuplicateMatch {
                    product: existing_product.clone(),
                    similarity_score,
                    algorithm_scores,
                    confidence,
                });
            }
        }

        // Sort matches by similarity score (highest first)
        matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        self.transaction_dao.commit(tx).await?;
        Ok(matches)
    }
}
