use std::sync::Arc;
use async_trait::async_trait;
use mockall::automock;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{permission::Authentication, ServiceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price {
    cents: i64,  // Store price in cents to avoid floating point issues
}

impl Price {
    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }
    
    pub fn from_euros(euros: f64) -> Self {
        Self {
            cents: (euros * 100.0).round() as i64,
        }
    }
    
    pub fn to_cents(&self) -> i64 {
        self.cents
    }
    
    pub fn to_euros(&self) -> f64 {
        self.cents as f64 / 100.0
    }
}

// For database storage/retrieval
impl From<i64> for Price {
    fn from(cents: i64) -> Self {
        Self::from_cents(cents)
    }
}

impl From<Price> for i64 {
    fn from(price: Price) -> Self {
        price.cents
    }
}

// For CSV parsing (German format "5,39")
impl std::str::FromStr for Price {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.replace(',', ".");
        let euros = normalized.parse::<f64>()
            .map_err(|e| format!("Invalid price format: {}", e))?;
        Ok(Self::from_euros(euros))
    }
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: Uuid,
    pub ean: Arc<str>,
    pub name: Arc<str>,
    pub short_name: Arc<str>,
    pub sales_unit: Arc<str>,
    pub requires_weighing: bool,
    pub price: Price,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&inventurly_dao::product::ProductEntity> for Product {
    fn from(entity: &inventurly_dao::product::ProductEntity) -> Self {
        Self {
            id: entity.id,
            ean: entity.ean.clone(),
            name: entity.name.clone(),
            short_name: entity.short_name.clone(),
            sales_unit: entity.sales_unit.clone(),
            requires_weighing: entity.requires_weighing,
            price: Price::from_cents(entity.price),
            created: entity.created,
            deleted: entity.deleted,
            version: entity.version,
        }
    }
}

impl From<&Product> for inventurly_dao::product::ProductEntity {
    fn from(product: &Product) -> Self {
        Self {
            id: product.id,
            ean: product.ean.clone(),
            name: product.name.clone(),
            short_name: product.short_name.clone(),
            sales_unit: product.sales_unit.clone(),
            requires_weighing: product.requires_weighing,
            price: product.price.to_cents(),
            created: product.created,
            deleted: product.deleted,
            version: product.version,
        }
    }
}

#[automock(type Context = MockContext; type Transaction = MockTransaction;)]
#[async_trait]
pub trait ProductService: Send + Sync {
    type Context: Send + Sync;
    type Transaction: Send + Sync;
    
    async fn get_all(
        &self,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Product]>, ServiceError>;
    
    async fn get_by_ean(
        &self,
        ean: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError>;
    
    async fn get_by_id(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError>;
    
    async fn create(
        &self,
        item: &Product,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError>;
    
    async fn update(
        &self,
        item: &Product,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Product, ServiceError>;
    
    async fn delete(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(), ServiceError>;
    
    async fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[Product]>, ServiceError>;
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