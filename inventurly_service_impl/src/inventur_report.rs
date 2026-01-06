use async_trait::async_trait;
use csv::Writer;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use inventurly_dao::{
    inventur_custom_entry::InventurCustomEntryDao, inventur_measurement::InventurMeasurementDao,
    product::ProductDao, rack::RackDao, TransactionDao,
};
use inventurly_service::{
    inventur_report::{InventurProductReportItem, InventurReportService},
    permission::{Authentication, PermissionService, ADMIN_PRIVILEGE},
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct InventurReportServiceImpl: InventurReportService = InventurReportServiceDeps {
        InventurMeasurementDao: InventurMeasurementDao<Transaction = Self::Transaction> = inventur_measurement_dao,
        InventurCustomEntryDao: InventurCustomEntryDao<Transaction = Self::Transaction> = inventur_custom_entry_dao,
        ProductDao: ProductDao<Transaction = Self::Transaction> = product_dao,
        RackDao: RackDao<Transaction = Self::Transaction> = rack_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

/// Intermediate structure for aggregating data by EAN
#[derive(Debug, Default)]
struct AggregatedData {
    product_name: Arc<str>,
    short_name: Arc<str>,
    total_count: Option<i64>,
    total_weight_grams: Option<i64>,
    measurement_count: usize,
    racks_measured: Vec<Arc<str>>,
}

impl AggregatedData {
    fn add_count(&mut self, count: Option<i64>) {
        if let Some(c) = count {
            self.total_count = Some(self.total_count.unwrap_or(0) + c);
        }
    }

    fn add_weight(&mut self, weight: Option<i64>) {
        if let Some(w) = weight {
            self.total_weight_grams = Some(self.total_weight_grams.unwrap_or(0) + w);
        }
    }

    fn add_rack(&mut self, rack_name: Arc<str>) {
        if !self.racks_measured.contains(&rack_name) {
            self.racks_measured.push(rack_name);
        }
    }
}

#[async_trait]
impl<Deps: InventurReportServiceDeps> InventurReportService for InventurReportServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn get_product_report(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[InventurProductReportItem]>, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;

        // Check permission - must have access via claims or be admin
        self.permission_service
            .check_inventur_permission(ADMIN_PRIVILEGE, inventur_id, context)
            .await?;

        // Load all data
        let measurements = self
            .inventur_measurement_dao
            .find_by_inventur_id(inventur_id, tx.clone())
            .await?;

        let custom_entries = self
            .inventur_custom_entry_dao
            .find_by_inventur_id(inventur_id, tx.clone())
            .await?;

        let products = self.product_dao.all(tx.clone()).await?;
        let racks = self.rack_dao.all(tx.clone()).await?;

        // Build lookup maps
        let product_by_id: HashMap<Uuid, _> = products.iter().map(|p| (p.id, p)).collect();
        let rack_by_id: HashMap<Uuid, _> = racks.iter().map(|r| (r.id, r)).collect();
        let product_by_ean: HashMap<&str, _> =
            products.iter().map(|p| (p.ean.as_ref(), p)).collect();

        // Aggregate by EAN
        let mut aggregated: HashMap<Arc<str>, AggregatedData> = HashMap::new();

        // Process measurements
        for measurement in measurements.iter() {
            if let Some(product) = product_by_id.get(&measurement.product_id) {
                let ean: Arc<str> = product.ean.clone();
                let entry = aggregated.entry(ean).or_insert_with(|| AggregatedData {
                    product_name: product.name.clone(),
                    short_name: product.short_name.clone(),
                    ..Default::default()
                });

                entry.add_count(measurement.count);
                entry.add_weight(measurement.weight_grams);
                entry.measurement_count += 1;

                if let Some(rack_id) = measurement.rack_id {
                    if let Some(rack) = rack_by_id.get(&rack_id) {
                        entry.add_rack(rack.name.clone());
                    }
                }
            }
        }

        // Process custom entries (only those with EAN)
        for custom_entry in custom_entries.iter() {
            if let Some(ean) = &custom_entry.ean {
                let ean: Arc<str> = ean.clone();

                // Try to get product info from the EAN
                let (product_name, short_name) = if let Some(product) = product_by_ean.get(ean.as_ref()) {
                    (product.name.clone(), product.short_name.clone())
                } else {
                    // Use custom product name if no matching product
                    (
                        custom_entry.custom_product_name.clone(),
                        custom_entry.custom_product_name.clone(),
                    )
                };

                let entry = aggregated.entry(ean).or_insert_with(|| AggregatedData {
                    product_name,
                    short_name,
                    ..Default::default()
                });

                entry.add_count(custom_entry.count);
                entry.add_weight(custom_entry.weight_grams);
                entry.measurement_count += 1;

                if let Some(rack_id) = custom_entry.rack_id {
                    if let Some(rack) = rack_by_id.get(&rack_id) {
                        entry.add_rack(rack.name.clone());
                    }
                }
            }
        }

        // Convert to result items and sort by EAN
        let mut items: Vec<InventurProductReportItem> = aggregated
            .into_iter()
            .map(|(ean, data)| InventurProductReportItem {
                ean,
                product_name: data.product_name,
                short_name: data.short_name,
                total_count: data.total_count,
                total_weight_grams: data.total_weight_grams,
                measurement_count: data.measurement_count,
                racks_measured: data.racks_measured,
            })
            .collect();

        items.sort_by(|a, b| a.ean.cmp(&b.ean));

        self.transaction_dao.commit(tx).await?;

        Ok(items.into())
    }

    async fn get_product_report_csv(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<String, ServiceError> {
        // Get the report data
        let items = self.get_product_report(inventur_id, context, tx).await?;

        // Generate CSV
        let mut wtr = Writer::from_writer(vec![]);

        // Write header
        wtr.write_record([
            "EAN",
            "Product Name",
            "Short Name",
            "Count",
            "Weight (g)",
            "Measurements",
            "Racks",
        ])
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("CSV write error: {}", e))))?;

        // Write data rows
        for item in items.iter() {
            wtr.write_record([
                item.ean.as_ref(),
                item.product_name.as_ref(),
                item.short_name.as_ref(),
                &item
                    .total_count
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
                &item
                    .total_weight_grams
                    .map(|w| w.to_string())
                    .unwrap_or_default(),
                &item.measurement_count.to_string(),
                &item
                    .racks_measured
                    .iter()
                    .map(|r| r.as_ref())
                    .collect::<Vec<&str>>()
                    .join(", "),
            ])
            .map_err(|e| {
                ServiceError::InternalError(Arc::from(format!("CSV write error: {}", e)))
            })?;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| ServiceError::InternalError(Arc::from(format!("CSV error: {}", e))))?;

        String::from_utf8(data)
            .map_err(|e| ServiceError::InternalError(Arc::from(format!("UTF-8 error: {}", e))))
    }
}
