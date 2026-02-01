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
    inventur_report::{InventurProductReportItem, InventurReportService, InventurStatistics},
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
    price_cents: Option<i64>,
    total_value_cents: Option<i64>,
    requires_weighing: Option<bool>,
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

    fn add_value(&mut self, count: Option<i64>, weight_grams: Option<i64>) {
        if let (Some(price), Some(requires_weighing)) = (self.price_cents, self.requires_weighing) {
            if requires_weighing {
                // Price is always per kg (1000g), measurement is always in grams
                if let Some(weight) = weight_grams {
                    let value = (weight as f64 / 1000.0) * price as f64;
                    self.total_value_cents = Some(
                        self.total_value_cents.unwrap_or(0) + value.round() as i64
                    );
                }
            } else if let Some(count) = count {
                self.total_value_cents = Some(
                    self.total_value_cents.unwrap_or(0) + count * price
                );
            }
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
                    price_cents: Some(product.price),
                    requires_weighing: Some(product.requires_weighing),
                    ..Default::default()
                });

                entry.add_count(measurement.count);
                entry.add_weight(measurement.weight_grams);
                entry.add_value(measurement.count, measurement.weight_grams);
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
                let (product_name, short_name, price_cents, requires_weighing) =
                    if let Some(product) = product_by_ean.get(ean.as_ref()) {
                        (
                            product.name.clone(),
                            product.short_name.clone(),
                            Some(product.price),
                            Some(product.requires_weighing),
                        )
                    } else {
                        // Use custom product name if no matching product, no price available
                        (
                            custom_entry.custom_product_name.clone(),
                            custom_entry.custom_product_name.clone(),
                            None,
                            None,
                        )
                    };

                let entry = aggregated.entry(ean).or_insert_with(|| AggregatedData {
                    product_name,
                    short_name,
                    price_cents,
                    requires_weighing,
                    ..Default::default()
                });

                entry.add_count(custom_entry.count);
                entry.add_weight(custom_entry.weight_grams);
                entry.add_value(custom_entry.count, custom_entry.weight_grams);
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
                price_cents: data.price_cents,
                total_value_cents: data.total_value_cents,
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
            "Price / kg (EUR)",
            "Total Value (EUR)",
        ])
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("CSV write error: {}", e))))?;

        // Write data rows
        for item in items.iter() {
            // Format prices as euros with 2 decimal places
            let price_str = item
                .price_cents
                .map(|cents| format!("{:.2}", cents as f64 / 100.0))
                .unwrap_or_default();
            let total_value_str = item
                .total_value_cents
                .map(|cents| format!("{:.2}", cents as f64 / 100.0))
                .unwrap_or_default();

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
                &price_str,
                &total_value_str,
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

    async fn get_statistics(
        &self,
        inventur_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<InventurStatistics, ServiceError> {
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

        // Build lookup maps
        let product_by_id: HashMap<Uuid, _> = products.iter().map(|p| (p.id, p)).collect();
        let product_by_ean: HashMap<&str, _> =
            products.iter().map(|p| (p.ean.as_ref(), p)).collect();

        let mut total_value_cents: i64 = 0;
        let mut total_entries: usize = 0;
        let mut products_with_positive_entries: std::collections::HashSet<Arc<str>> =
            std::collections::HashSet::new();

        // Process measurements
        for measurement in measurements.iter() {
            if measurement.deleted.is_some() {
                continue;
            }
            total_entries += 1;

            if let Some(product) = product_by_id.get(&measurement.product_id) {
                // Check if this is a positive entry
                let has_positive_count = measurement.count.map(|c| c > 0).unwrap_or(false);
                let has_positive_weight = measurement.weight_grams.map(|w| w > 0).unwrap_or(false);

                if has_positive_count || has_positive_weight {
                    products_with_positive_entries.insert(product.ean.clone());
                }

                // Calculate value
                if product.requires_weighing {
                    // Weight-based: parse sales_unit to get reference weight
                    if let Some(weight_grams) = measurement.weight_grams {
                        if let Some(ref_weight_grams) = parse_sales_unit_to_grams(&product.sales_unit) {
                            if ref_weight_grams > 0 {
                                // value = (weight / ref_weight) * price
                                let value = (weight_grams as f64 / ref_weight_grams as f64) * product.price as f64;
                                total_value_cents += value.round() as i64;
                            }
                        }
                    }
                } else {
                    // Count-based: count * price
                    if let Some(count) = measurement.count {
                        total_value_cents += count * product.price;
                    }
                }
            }
        }

        // Process custom entries (only those with EAN for value calculation)
        for custom_entry in custom_entries.iter() {
            if custom_entry.deleted.is_some() {
                continue;
            }
            total_entries += 1;

            if let Some(ean) = &custom_entry.ean {
                // Check if this is a positive entry
                let has_positive_count = custom_entry.count.map(|c| c > 0).unwrap_or(false);
                let has_positive_weight = custom_entry.weight_grams.map(|w| w > 0).unwrap_or(false);

                if has_positive_count || has_positive_weight {
                    products_with_positive_entries.insert(ean.clone());
                }

                // Calculate value if product exists
                if let Some(product) = product_by_ean.get(ean.as_ref()) {
                    if product.requires_weighing {
                        // Weight-based
                        if let Some(weight_grams) = custom_entry.weight_grams {
                            if let Some(ref_weight_grams) = parse_sales_unit_to_grams(&product.sales_unit) {
                                if ref_weight_grams > 0 {
                                    let value = (weight_grams as f64 / ref_weight_grams as f64) * product.price as f64;
                                    total_value_cents += value.round() as i64;
                                }
                            }
                        }
                    } else {
                        // Count-based
                        if let Some(count) = custom_entry.count {
                            total_value_cents += count * product.price;
                        }
                    }
                }
            }
        }

        self.transaction_dao.commit(tx).await?;

        Ok(InventurStatistics {
            total_value_cents,
            total_entries,
            products_with_entries: products_with_positive_entries.len(),
        })
    }
}

/// Parse sales_unit string (e.g., "100g", "1kg", "250g") to grams
fn parse_sales_unit_to_grams(sales_unit: &str) -> Option<i64> {
    let sales_unit = sales_unit.trim().to_lowercase();

    // Try to extract number and unit
    let mut num_str = String::new();
    let mut unit_str = String::new();
    let mut found_digit = false;

    for c in sales_unit.chars() {
        if c.is_ascii_digit() || c == '.' || c == ',' {
            if !unit_str.is_empty() {
                // Digit after unit, invalid format
                return None;
            }
            num_str.push(if c == ',' { '.' } else { c });
            found_digit = true;
        } else if c.is_alphabetic() {
            unit_str.push(c);
        }
        // Skip whitespace and other characters
    }

    if !found_digit {
        return None;
    }

    let number: f64 = num_str.parse().ok()?;

    // Convert to grams based on unit
    let grams = match unit_str.as_str() {
        "g" | "gr" | "gram" | "grams" => number,
        "kg" | "kilo" | "kilogram" | "kilograms" => number * 1000.0,
        "" => number, // Assume grams if no unit
        _ => return None, // Unknown unit
    };

    Some(grams.round() as i64)
}
