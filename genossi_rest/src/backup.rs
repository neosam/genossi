use axum::extract::{Extension, Query, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{body::Body, Router};
use genossi_backup::generator;
use genossi_backup::webdav::WebDavClient;
use genossi_config::service::ConfigService;
use genossi_dao::backup::BackupDao;
use genossi_service::auth_types::privileges;
use genossi_service::document_storage::DocumentStorage;
use genossi_service::permission::{Authentication, PermissionService};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use tracing::instrument;

use crate::{error_handler, Context, RestError, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/members", get(export_members::<RestState>))
        .route("/actions", get(export_actions::<RestState>))
        .route("/documents", get(export_documents::<RestState>))
        .route("/test-webdav", post(test_webdav::<RestState>))
}

async fn require_export_backup<RestState: RestStateDef>(
    rest_state: &RestState,
    context: Context,
) -> Result<(), RestError> {
    let auth = crate::extract_auth_context(Some(context))?;
    let authentication: Authentication<_> = Authentication::from(auth);
    rest_state
        .permission_service()
        .check_permission(privileges::EXPORT_BACKUP, authentication)
        .await
        .map_err(RestError::from)
}

#[derive(Debug, Deserialize)]
pub struct MembersQuery {
    pub date: String,
}

#[instrument(skip(rest_state))]
pub async fn export_members<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    query: Query<MembersQuery>,
) -> Response {
    error_handler(
        (async {
            require_export_backup(&*rest_state, context).await?;

            let format = time::format_description::parse("[year]-[month]-[day]")
                .map_err(|e| RestError::InternalError(e.to_string()))?;
            let date = time::Date::parse(&query.date, &format)
                .map_err(|_| RestError::BadRequest("Invalid date format. Use YYYY-MM-DD.".into()))?;

            let members = rest_state
                .backup_dao()
                .members_at_date(date)
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let buf = generator::generate_members_csv(&members)
                .map_err(RestError::InternalError)?;

            let filename = format!("mitgliederliste_{}.csv", query.date);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/csv; charset=utf-8")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(Body::from(buf))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
pub async fn export_actions<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            require_export_backup(&*rest_state, context).await?;

            let actions = rest_state
                .backup_dao()
                .all_actions()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let buf = generator::generate_actions_csv(&actions)
                .map_err(RestError::InternalError)?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/csv; charset=utf-8")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"aktionen.csv\"",
                )
                .body(Body::from(buf))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
pub async fn export_documents<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            require_export_backup(&*rest_state, context).await?;

            let documents = rest_state
                .backup_dao()
                .all_documents()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let communications = rest_state
                .backup_dao()
                .all_communications()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let document_storage = rest_state.document_storage();

            let mut zip_buf = Cursor::new(Vec::new());
            {
                let mut zip = zip::ZipWriter::new(&mut zip_buf);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);

                for doc in documents.iter() {
                    let data = match document_storage.load(&doc.relative_path).await {
                        Ok(data) => data,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to load document {}: {:?}",
                                doc.relative_path,
                                e
                            );
                            continue;
                        }
                    };

                    let dir_name = format!(
                        "{:03}_{}_{}",
                        doc.member_number, doc.last_name, doc.first_name
                    );
                    let file_path =
                        format!("{}/{}_{}", dir_name, doc.document_type, doc.file_name);

                    zip.start_file(&file_path, options)
                        .map_err(|e| RestError::InternalError(e.to_string()))?;
                    zip.write_all(&data)
                        .map_err(|e| RestError::InternalError(e.to_string()))?;
                }

                // Add communication files grouped by member
                let mut filename_counts: HashMap<String, u32> = HashMap::new();
                for comm in communications.iter() {
                    let dir_name = format!(
                        "{:03}_{}_{}",
                        comm.member_number, comm.last_name, comm.first_name
                    );
                    let base_filename = generator::generate_communication_filename(
                        &comm.date,
                        &comm.direction,
                        &comm.subject,
                        None,
                    );

                    let full_base = format!("{}/kommunikation/{}", dir_name, base_filename);
                    let count = filename_counts.entry(full_base.clone()).or_insert(0);
                    *count += 1;

                    let file_path = if *count > 1 {
                        let suffix = &comm.mail_id.to_string()[..8];
                        let filename_with_suffix =
                            generator::generate_communication_filename(
                                &comm.date,
                                &comm.direction,
                                &comm.subject,
                                Some(suffix),
                            );
                        format!("{}/kommunikation/{}.txt", dir_name, filename_with_suffix)
                    } else {
                        format!("{}.txt", full_base)
                    };

                    let txt_content = generator::generate_communication_txt(comm);

                    zip.start_file(&file_path, options)
                        .map_err(|e| RestError::InternalError(e.to_string()))?;
                    zip.write_all(txt_content.as_bytes())
                        .map_err(|e| RestError::InternalError(e.to_string()))?;
                }

                zip.finish()
                    .map_err(|e| RestError::InternalError(e.to_string()))?;
            }

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/zip")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"dokumente.zip\"",
                )
                .body(Body::from(zip_buf.into_inner()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
pub async fn test_webdav<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            require_export_backup(&*rest_state, context).await?;

            let entries = rest_state
                .config_service()
                .get_all()
                .await
                .map_err(|e| RestError::InternalError(format!("{:?}", e)))?;

            let find = |key: &str| -> Option<String> {
                entries
                    .iter()
                    .find(|e| e.key.as_ref() == key)
                    .map(|e| e.value.to_string())
            };

            let url = find("backup_webdav_url")
                .ok_or_else(|| RestError::BadRequest("backup_webdav_url not configured".into()))?;
            let username = find("backup_webdav_username")
                .ok_or_else(|| RestError::BadRequest("backup_webdav_username not configured".into()))?;
            let password = find("backup_webdav_password")
                .ok_or_else(|| RestError::BadRequest("backup_webdav_password not configured".into()))?;
            let directory = find("backup_webdav_directory")
                .unwrap_or_else(|| "genossi-export".to_string());

            let client = WebDavClient::new(&url, &username, &password);
            client
                .test_connection(&directory)
                .await
                .map_err(|e| RestError::InternalError(format!("{}", e)))?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::json!({"success": true}).to_string(),
                ))
                .unwrap())
        })
        .await,
    )
}
