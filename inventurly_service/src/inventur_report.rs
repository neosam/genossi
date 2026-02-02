use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

/// Rack information for report items (ID and name)
#[derive(Debug, Clone)]
pub struct RackMeasured {
    pub id: Uuid,
    pub name: Arc<str>,
}

/// Aggregated product measurement data for an inventur report
#[derive(Debug, Clone)]
pub struct InventurProductReportItem {
    /// Product UUID (None for custom entries without linked product)
    pub product_id: Option<Uuid>,
    pub ean: Arc<str>,
    pub product_name: Arc<str>,
    pub short_name: Arc<str>,
    pub total_count: Option<i64>,
    pub total_weight_grams: Option<i64>,
    pub measurement_count: usize,
    pub racks_measured: Vec<RackMeasured>,
    /// Unit price in cents (None if product not found in database)
    pub price_cents: Option<i64>,
    /// Calculated total value in cents based on count/weight (None if can't calculate)
    pub total_value_cents: Option<i64>,
}

/// Statistics summary for an inventur
#[derive(Debug, Clone)]
pub struct InventurStatistics {
    /// Total monetary value in cents
    pub total_value_cents: i64,
    /// Total number of measurements + custom entries
    pub total_entries: usize,
    /// Number of distinct products with at least one positive entry
    pub products_with_entries: usize,
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait InventurReportService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;

    /// Generate cumulative product report for an inventur
    /// Aggregates all measurements and custom entries (with EAN) by product EAN
    async fn get_product_report(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurProductReportItem]>, ServiceError>;

    /// Generate CSV string for the product report
    async fn get_product_report_csv(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<String, ServiceError>;

    /// Get statistics summary for an inventur
    async fn get_statistics(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurStatistics, ServiceError>;
}

mockall::mock! {
    pub Context {}
    impl Clone for Context {
        fn clone(&self) -> Self;
    }
    unsafe impl Send for Context {}
    unsafe impl Sync for Context {}
}

mockall::mock! {
    pub Transaction {}
    impl Clone for Transaction {
        fn clone(&self) -> Self;
    }
    unsafe impl Send for Transaction {}
    unsafe impl Sync for Transaction {}
}
