use axum::extract::{Extension, Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::{body::Body, Router};
use genossi_dao::backup::BackupDao;
use genossi_service::auth_types::privileges;
use genossi_service::document_storage::DocumentStorage;
use genossi_service::permission::{Authentication, PermissionService};
use serde::Deserialize;
use std::io::{Cursor, Write};
use tracing::instrument;

use crate::{error_handler, Context, RestError, RestStateDef};

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/members", get(export_members::<RestState>))
        .route("/actions", get(export_actions::<RestState>))
        .route("/documents", get(export_documents::<RestState>))
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

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

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

            let mut buf = Vec::new();
            buf.extend_from_slice(UTF8_BOM);

            {
                let mut wtr = csv::Writer::from_writer(&mut buf);
                wtr.write_record([
                    "Mitgliedsnummer",
                    "Anrede",
                    "Titel",
                    "Vorname",
                    "Nachname",
                    "Firma",
                    "Strasse",
                    "Hausnummer",
                    "PLZ",
                    "Ort",
                    "Email",
                    "Bankverbindung",
                    "Beitrittsdatum",
                    "Austrittsdatum",
                    "Anteile bei Beitritt",
                    "Anteile am Stichtag",
                    "Kommentar",
                ])
                .map_err(|e| RestError::InternalError(e.to_string()))?;

                for m in members.iter() {
                    wtr.write_record([
                        m.member_number.to_string(),
                        m.salutation.as_deref().unwrap_or("").to_string(),
                        m.title.as_deref().unwrap_or("").to_string(),
                        m.first_name.to_string(),
                        m.last_name.to_string(),
                        m.company.as_deref().unwrap_or("").to_string(),
                        m.street.as_deref().unwrap_or("").to_string(),
                        m.house_number.as_deref().unwrap_or("").to_string(),
                        m.postal_code.as_deref().unwrap_or("").to_string(),
                        m.city.as_deref().unwrap_or("").to_string(),
                        m.email.as_deref().unwrap_or("").to_string(),
                        m.bank_account.as_deref().unwrap_or("").to_string(),
                        m.join_date.to_string(),
                        m.exit_date.as_deref().unwrap_or("").to_string(),
                        m.shares_at_joining.to_string(),
                        m.shares_at_date.to_string(),
                        m.comment.as_deref().unwrap_or("").to_string(),
                    ])
                    .map_err(|e| RestError::InternalError(e.to_string()))?;
                }

                wtr.flush()
                    .map_err(|e| RestError::InternalError(e.to_string()))?;
            }

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

            let mut buf = Vec::new();
            buf.extend_from_slice(UTF8_BOM);

            {
                let mut wtr = csv::Writer::from_writer(&mut buf);
                wtr.write_record([
                    "Mitgliedsnummer",
                    "Vorname",
                    "Nachname",
                    "Aktionstyp",
                    "Datum",
                    "Anteileaenderung",
                    "Uebertragung-Mitgliedsnummer",
                    "Wirksamkeitsdatum",
                    "Kommentar",
                ])
                .map_err(|e| RestError::InternalError(e.to_string()))?;

                for a in actions.iter() {
                    wtr.write_record([
                        a.member_number.to_string(),
                        a.first_name.to_string(),
                        a.last_name.to_string(),
                        a.action_type.to_string(),
                        a.date.to_string(),
                        a.shares_change.to_string(),
                        a.transfer_member_number
                            .map(|n| n.to_string())
                            .unwrap_or_default(),
                        a.effective_date.as_deref().unwrap_or("").to_string(),
                        a.comment.as_deref().unwrap_or("").to_string(),
                    ])
                    .map_err(|e| RestError::InternalError(e.to_string()))?;
                }

                wtr.flush()
                    .map_err(|e| RestError::InternalError(e.to_string()))?;
            }

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
