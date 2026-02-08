use axum::body::Body;
use axum::extract::{Multipart, State};
use axum::response::Response;
use axum::routing::post;
use axum::{Extension, Router};
use inventurly_service::deposit_ean_import::{
    DepositEanImportError, DepositEanImportResult, DepositEanImportService,
};
use tracing::instrument;
use utoipa::OpenApi;

use crate::{error_handler, Context, RestError, RestStateDef};

/// Schema for CSV file upload in multipart form
#[derive(utoipa::ToSchema)]
#[schema(title = "DepositEanCsvFileUpload")]
pub struct DepositEanCsvFileUpload {
    /// CSV file to upload with EAN and PfandEAN columns (semicolon-separated)
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new().route("/import", post(import_deposit_eans_csv::<RestState>))
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "Deposit EAN Import",
    path = "/import",
    request_body(
        content = inline(DepositEanCsvFileUpload),
        description = "CSV file upload with EAN and PfandEAN columns (semicolon-separated)",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "Deposit EAN import successful", body = DepositEanImportResult),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn import_deposit_eans_csv<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            // Extract the CSV file from multipart form data
            let mut csv_content = String::new();

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Multipart parsing error: {}", e)))?
            {
                let field_name = field.name().unwrap_or("unknown");

                if field_name == "file" {
                    let file_data = field.bytes().await.map_err(|e| {
                        RestError::BadRequest(format!("Failed to read file: {}", e))
                    })?;

                    csv_content = String::from_utf8(file_data.to_vec()).map_err(|e| {
                        RestError::BadRequest(format!("File is not valid UTF-8: {}", e))
                    })?;
                    break;
                }
            }

            if csv_content.is_empty() {
                return Err(RestError::BadRequest(
                    "No file provided or file is empty".to_string(),
                ));
            }

            // Import the CSV content
            let result: DepositEanImportResult = rest_state
                .deposit_ean_import_service()
                .import_deposit_eans_csv(
                    &csv_content,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await
                .map_err(RestError::from)?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&result).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(import_deposit_eans_csv),
    components(schemas(DepositEanImportResult, DepositEanImportError, DepositEanCsvFileUpload)),
    tags(
        (name = "Deposit EAN Import", description = "CSV deposit EAN import endpoints")
    )
)]
pub struct DepositEanImportApiDoc;
