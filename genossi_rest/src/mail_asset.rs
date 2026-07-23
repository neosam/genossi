//! Phase 27 (IMG-02/04): REST layer for inline mail image assets. Two
//! admin-only endpoints delegate to the [`MailAssetService`]:
//!
//! - `POST /`          — Multipart upload; returns `201` + `MailAssetTO`.
//! - `GET  /{id}/bytes` — Inline preview; returns the raw bytes with the
//!                        server-derived `Content-Type`.
//!
//! All permission enforcement lives in the Service layer (CR-02 ordering is
//! pinned there); the handlers just extract `Authentication<Context>` and
//! delegate. MIME validation is a magic-byte sniff in the service — the handler
//! deliberately does NOT extension-validate; it passes the raw bytes straight
//! through.

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use genossi_rest_types::MailAssetTO;
use genossi_service::mail_asset::{MailAssetService, UploadMailAsset};
use genossi_service::ServiceError;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

/// Multipart schema for the upload endpoint. Only the `file` field is consumed.
#[derive(ToSchema)]
#[allow(dead_code)]
struct MailAssetUpload {
    /// The image bytes (PNG/JPEG/GIF).
    #[schema(format = Binary)]
    file: String,
}

/// Body-Limit for uploads: 5 MB per image (IMG-02).
const MAIL_ASSET_BODY_LIMIT: usize = 5 * 1024 * 1024;

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            post(upload_mail_asset::<RestState>)
                .layer(DefaultBodyLimit::max(MAIL_ASSET_BODY_LIMIT)),
        )
        .route("/{id}/bytes", get(download_mail_asset_bytes::<RestState>))
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "Mail Assets",
    path = "",
    request_body(
        content_type = "multipart/form-data",
        content = MailAssetUpload,
        description = "The inline image (PNG/JPEG/GIF, max 5 MB)"
    ),
    responses(
        (status = 201, description = "Asset uploaded", body = MailAssetTO),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 415, description = "Unsupported file type"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn upload_mail_asset<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            let mut file_name: Option<String> = None;
            let mut file_data: Option<Vec<u8>> = None;

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Failed to read multipart: {}", e)))?
            {
                let name = field.name().unwrap_or("").to_string();
                if name.as_str() == "file" {
                    file_name = field.file_name().map(|s| s.to_string());
                    // Raw bytes only — the service magic-byte-sniffs the MIME.
                    file_data = Some(
                        field
                            .bytes()
                            .await
                            .map_err(|e| {
                                RestError::BadRequest(format!("Failed to read file: {}", e))
                            })?
                            .to_vec(),
                    );
                }
            }

            let data =
                file_data.ok_or_else(|| RestError::BadRequest("file is required".to_string()))?;
            let fname = file_name.unwrap_or_else(|| "image".to_string());

            let upload = UploadMailAsset {
                filename: fname,
                // Client MIME is untrusted and unused — the service sniffs it.
                mime_type: String::new(),
                data,
            };

            let asset = rest_state
                .mail_asset_service()
                .upload(upload, crate::extract_auth_context(Some(context))?, None)
                .await
                .map_err(map_upload_error)?;

            let to = MailAssetTO::from(&asset);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

/// Map the service's validation error (unsupported MIME) to 415; other errors
/// flow through the standard `ServiceError` → `RestError` conversion.
fn map_upload_error(err: ServiceError) -> RestError {
    match err {
        // A validation error on the "file" field with the unsupported-type
        // message is a bad media type (415), not a generic 422/400.
        ServiceError::ValidationError(ref items)
            if items
                .iter()
                .any(|i| i.message.contains("Unsupported image type")) =>
        {
            RestError::UnsupportedMediaType(
                serde_json::json!({
                    "error": "Unsupported image type — only PNG, JPEG and GIF are allowed",
                })
                .to_string(),
            )
        }
        other => RestError::from(other),
    }
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Mail Assets",
    path = "/{id}/bytes",
    params(
        ("id" = Uuid, Path, description = "Mail asset ID"),
    ),
    responses(
        (status = 200, description = "Asset bytes with server-derived Content-Type"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Asset not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn download_mail_asset_bytes<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let (asset, bytes) = rest_state
                .mail_asset_service()
                .download(id, crate::extract_auth_context(Some(context))?, None)
                .await?;

            // Inline preview — no Content-Disposition attachment header.
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", asset.mime_type.as_ref())
                .body(Body::from(bytes))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(upload_mail_asset, download_mail_asset_bytes),
    components(schemas(MailAssetTO, MailAssetUpload)),
    tags((name = "Mail Assets", description = "Inline image assets for HTML mail"))
)]
pub struct ApiDoc;
