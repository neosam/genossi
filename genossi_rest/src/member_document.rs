use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    response::Response,
    routing::{delete, get, post},
    Extension, Router,
};
use genossi_rest_types::MemberDocumentTO;
use genossi_service::document_storage::DocumentStorage;
use genossi_service::member::MemberService;
use genossi_service::member_document::{DocumentType, MemberDocumentService, UploadDocument};
use genossi_service::template::TemplateError;
use std::sync::Arc;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

/// Multipart file upload schema for document uploads
#[derive(ToSchema)]
#[allow(dead_code)]
struct DocumentUpload {
    /// The document type (join_declaration, join_confirmation, share_increase, other)
    document_type: String,
    /// Description (required for type 'other')
    description: Option<String>,
    /// The file to upload
    #[schema(format = Binary)]
    file: String,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_documents::<RestState>))
        .route("/", post(upload_document::<RestState>))
        .route(
            "/generate/{document_type}",
            post(generate_document::<RestState>),
        )
        .route("/{document_id}", get(download_document::<RestState>))
        .route("/{document_id}", delete(delete_document::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Member Documents",
    path = "",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
    ),
    responses(
        (status = 200, description = "List documents for member", body = [MemberDocumentTO]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_documents<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let docs: Arc<[MemberDocumentTO]> = rest_state
                .member_document_service()
                .list(member_id, crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(MemberDocumentTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&docs).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state, multipart))]
#[utoipa::path(
    post,
    tag = "Member Documents",
    path = "",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
    ),
    request_body(content_type = "multipart/form-data", content = DocumentUpload, description = "Document file with metadata"),
    responses(
        (status = 201, description = "Document uploaded", body = MemberDocumentTO),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn upload_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Response {
    error_handler(
        (async {
            let mut document_type: Option<String> = None;
            let mut description: Option<String> = None;
            let mut file_name: Option<String> = None;
            let mut mime_type: Option<String> = None;
            let mut file_data: Option<Vec<u8>> = None;

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| RestError::BadRequest(format!("Failed to read multipart: {}", e)))?
            {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "document_type" => {
                        document_type = Some(
                            field
                                .text()
                                .await
                                .map_err(|e| RestError::BadRequest(e.to_string()))?,
                        );
                    }
                    "description" => {
                        description = Some(
                            field
                                .text()
                                .await
                                .map_err(|e| RestError::BadRequest(e.to_string()))?,
                        );
                    }
                    "file" => {
                        file_name = field.file_name().map(|s| s.to_string());
                        mime_type = field.content_type().map(|s| s.to_string());
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
                    _ => {}
                }
            }

            let doc_type_str = document_type
                .ok_or_else(|| RestError::BadRequest("document_type is required".to_string()))?;
            let doc_type = DocumentType::from_str(&doc_type_str)
                .ok_or_else(|| RestError::BadRequest(format!("Invalid document_type: {}", doc_type_str)))?;
            let data = file_data
                .ok_or_else(|| RestError::BadRequest("file is required".to_string()))?;
            let fname = file_name.unwrap_or_else(|| "document".to_string());
            let mtype = mime_type.unwrap_or_else(|| "application/octet-stream".to_string());

            let upload = UploadDocument {
                member_id,
                document_type: doc_type,
                description,
                file_name: fname,
                mime_type: mtype,
                data: data.clone(),
            };

            let doc = rest_state
                .member_document_service()
                .upload(upload, crate::extract_auth_context(Some(context))?, None)
                .await?;

            // Save file to storage
            rest_state
                .document_storage()
                .save(&doc.relative_path, &data)
                .await
                .map_err(|e| RestError::InternalError(format!("Failed to save file: {}", e)))?;

            let to = MemberDocumentTO::from(&doc);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Member Documents",
    path = "/{document_id}",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
        ("document_id" = Uuid, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Document file", content_type = "application/octet-stream"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn download_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((member_id, document_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let (doc, _) = rest_state
                .member_document_service()
                .download(
                    member_id,
                    document_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;

            let data = rest_state
                .document_storage()
                .load(&doc.relative_path)
                .await
                .map_err(|e| RestError::InternalError(format!("Failed to load file: {}", e)))?;

            let content_disposition =
                format!("attachment; filename=\"{}\"", doc.file_name);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", doc.mime_type.as_ref())
                .header("Content-Disposition", content_disposition)
                .body(Body::from(data))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "Member Documents",
    path = "/{document_id}",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
        ("document_id" = Uuid, Path, description = "Document ID"),
    ),
    responses(
        (status = 204, description = "Document deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((member_id, document_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .member_document_service()
                .delete(
                    member_id,
                    document_id,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Member Documents",
    path = "/generate/{document_type}",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
        ("document_type" = String, Path, description = "Document type identifier (e.g. join_confirmation)"),
    ),
    responses(
        (status = 201, description = "Document generated and stored", body = MemberDocumentTO),
        (status = 400, description = "Invalid document type or template error"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member or template not found"),
        (status = 409, description = "Document of this type already exists"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn generate_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((member_id, document_type_str)): Path<(Uuid, String)>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;

            // Resolve document type from identifier
            let doc_type = DocumentType::from_str(&document_type_str).ok_or_else(|| {
                RestError::BadRequest(format!("Unknown document type: {}", document_type_str))
            })?;

            // Get template path from mapping
            let template_path = doc_type.template_path().ok_or_else(|| {
                RestError::BadRequest(format!(
                    "Document type '{}' does not support generation",
                    document_type_str
                ))
            })?;

            // Get member data
            let member = rest_state
                .member_service()
                .get(member_id, auth.clone(), None)
                .await
                .map_err(RestError::from)?;

            // Render PDF
            let pdf_bytes = rest_state
                .pdf_generator()
                .render(
                    template_path,
                    rest_state.template_storage().base_path(),
                    &member,
                )
                .map_err(|e| match e {
                    TemplateError::NotFound => RestError::NotFound,
                    TemplateError::RenderError(msg) => RestError::BadRequest(msg.to_string()),
                    other => RestError::InternalError(format!("{:?}", other)),
                })?;

            // Derive filename: e.g. "join_confirmation_1001_mustermann_max.pdf"
            let base_name = template_path.replace(".typ", "");
            let filename = format!(
                "{}_{}_{}_{}.pdf",
                base_name,
                member.member_number,
                member.last_name.to_lowercase(),
                member.first_name.to_lowercase(),
            );

            // Upload as MemberDocument (singleton check happens in service)
            let upload = UploadDocument {
                member_id,
                document_type: doc_type,
                description: None,
                file_name: filename,
                mime_type: "application/pdf".to_string(),
                data: pdf_bytes.clone(),
            };

            let doc = rest_state
                .member_document_service()
                .upload(upload, auth, None)
                .await?;

            // Save file to storage
            rest_state
                .document_storage()
                .save(&doc.relative_path, &pdf_bytes)
                .await
                .map_err(|e| RestError::InternalError(format!("Failed to save file: {}", e)))?;

            let to = MemberDocumentTO::from(&doc);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(list_documents, upload_document, download_document, delete_document, generate_document),
    components(schemas(MemberDocumentTO, DocumentUpload)),
    tags((name = "Member Documents", description = "Member document management endpoints"))
)]
pub struct ApiDoc;
