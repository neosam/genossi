use std::sync::Arc;
use async_trait::async_trait;
use csv::ReaderBuilder;

use inventurly_dao::TransactionDao;
use inventurly_service::{
    permission::{Authentication, ADMIN_PRIVILEGE, PermissionService},
    product::ProductService,
    csv_import::{CsvImportService, CsvImportResult, CsvImportError, CsvProductRow, ImportAction},
    ServiceError,
};

use crate::gen_service_impl;

gen_service_impl! {
    struct CsvImportServiceImpl: CsvImportService = CsvImportServiceDeps {
        ProductService: ProductService<Context = Self::Context, Transaction = Self::Transaction> = product_service,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

const CSV_IMPORT_SERVICE_PROCESS: &str = "csv-import-service";

impl<Deps: CsvImportServiceDeps> CsvImportServiceImpl<Deps> {
    /// Parse CSV content and extract product rows
    fn parse_csv_content(&self, csv_content: &str) -> Result<Vec<CsvProductRow>, String> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_content.as_bytes());
        
        let headers = reader.headers()
            .map_err(|e| format!("Failed to read CSV headers: {}", e))?;
        
        // Find column indices based on the expected German headers
        let ean_idx = headers.iter().position(|h| h == "EAN")
            .ok_or("EAN column not found")?;
        let bezeichnung_idx = headers.iter().position(|h| h == "Bezeichnung")
            .ok_or("Bezeichnung column not found")?;
        let kurzbezeichnung_idx = headers.iter().position(|h| h == "Kurzbezeichnung")
            .ok_or("Kurzbezeichnung column not found")?;
        let vk_einheit_idx = headers.iter().position(|h| h == "VKEinheit")
            .ok_or("VKEinheit column not found")?;
        let wiege_artikel_idx = headers.iter().position(|h| h == "WiegeArtikel")
            .ok_or("WiegeArtikel column not found")?;
        let vk_herst_idx = headers.iter().position(|h| h == "VKHerst")
            .ok_or("VKHerst column not found")?;
        
        let mut rows = Vec::new();
        
        for (row_number, result) in reader.records().enumerate() {
            let record = result
                .map_err(|e| format!("Failed to read row {}: {}", row_number + 2, e))?;
            
            // Skip rows with empty EAN (likely incomplete data)
            let ean = record.get(ean_idx).unwrap_or("").trim();
            if ean.is_empty() {
                continue;
            }
            
            let row = CsvProductRow {
                ean: ean.to_string(),
                bezeichnung: record.get(bezeichnung_idx).unwrap_or("").trim().to_string(),
                kurzbezeichnung: record.get(kurzbezeichnung_idx).unwrap_or("").trim().to_string(),
                vk_einheit: record.get(vk_einheit_idx).unwrap_or("").trim().to_string(),
                wiege_artikel: record.get(wiege_artikel_idx).unwrap_or("0").trim().to_string(),
                vk_herst: record.get(vk_herst_idx).unwrap_or("0,00").trim().to_string(),
            };
            
            rows.push(row);
        }
        
        Ok(rows)
    }
}

#[async_trait]
impl<Deps: CsvImportServiceDeps> CsvImportService for CsvImportServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn import_products_csv(
        &self,
        csv_content: &str,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<CsvImportResult, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        // Check permissions
        self.permission_service
            .check_permission(ADMIN_PRIVILEGE, context.clone())
            .await?;
        
        // Parse CSV content
        let rows = self.parse_csv_content(csv_content)
            .map_err(|e| ServiceError::ValidationError(vec![
                inventurly_service::ValidationFailureItem {
                    field: Arc::from("csv"),
                    message: Arc::from(e),
                }
            ]))?;
        
        let total_rows = rows.len();
        let mut created = 0;
        let mut updated = 0;
        let mut errors = Vec::new();
        
        // Process each row
        for (index, row) in rows.into_iter().enumerate() {
            let row_number = index + 2; // +2 because CSV has header and is 1-indexed
            let ean = row.ean.clone();
            
            match self.import_product_row(row, context.clone(), Some(tx.clone())).await {
                Ok(ImportAction::Created) => created += 1,
                Ok(ImportAction::Updated) => updated += 1,
                Err(e) => {
                    errors.push(CsvImportError {
                        row: row_number,
                        ean,
                        message: format!("{:?}", e),
                    });
                }
            }
        }
        
        // If we have too many errors, consider the import failed
        if errors.len() > total_rows / 2 {
            // Don't commit the transaction (rollback by not committing)
            return Err(ServiceError::ValidationError(vec![
                inventurly_service::ValidationFailureItem {
                    field: Arc::from("csv"),
                    message: Arc::from(format!("Too many errors ({}/{}) - import aborted", errors.len(), total_rows)),
                }
            ]));
        }
        
        self.transaction_dao.commit(tx).await?;
        
        Ok(CsvImportResult {
            total_rows,
            created,
            updated,
            errors,
        })
    }

    async fn import_product_row(
        &self,
        row: CsvProductRow,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<ImportAction, ServiceError> {
        let tx = self.transaction_dao.use_transaction(tx).await?;
        
        // Convert CSV row to Product
        let product = inventurly_service::product::Product::try_from(row)
            .map_err(|e| ServiceError::ValidationError(vec![
                inventurly_service::ValidationFailureItem {
                    field: Arc::from("product"),
                    message: Arc::from(e),
                }
            ]))?;
        
        // Check if product already exists
        match self.product_service.get_by_ean(&product.ean, context.clone(), Some(tx.clone())).await {
            Ok(existing) => {
                // Update existing product
                let updated_product = inventurly_service::product::Product {
                    id: existing.id,
                    version: existing.version,
                    created: existing.created,
                    deleted: existing.deleted,
                    ..product
                };
                
                self.product_service
                    .update(&updated_product, context, Some(tx.clone()))
                    .await?;
                
                self.transaction_dao.commit(tx).await?;
                Ok(ImportAction::Updated)
            }
            Err(ServiceError::EntityNotFound(_)) => {
                // Create new product
                self.product_service
                    .create(&product, context, Some(tx.clone()))
                    .await?;
                
                self.transaction_dao.commit(tx).await?;
                Ok(ImportAction::Created)
            }
            Err(e) => {
                // Don't commit the transaction (rollback by not committing)
                Err(e)
            }
        }
    }
}