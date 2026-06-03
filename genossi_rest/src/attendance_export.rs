//! REST handlers for the Attendance-Export aggregate (Phase 6 Plan 03).
//!
//! Route (D-14):
//!   * `GET /api/assembly/{assembly_id}/attendance-export/{format}` with
//!     optional query parameter `?include=all|present` (D-09).
//!
//! Permission decisions live in `AttendanceExportServiceImpl::check_admin_and_closed`
//! (Plan 02). The handler ONLY validates the format-suffix whitelist (D-14)
//! and maps `ServiceError` variants to HTTP status codes — it does NOT touch
//! any permission logic itself.
//!
//! D-13 / D-26 — local `map_export_error`:
//! `ServiceError::PermissionDenied` is mapped to `RestError::Forbidden(403)`
//! (analog zu `attendance.rs::map_attendance_error`). Damit kann das Frontend
//! "no admin privilege" (403) klar von "session invalid" (401) unterscheiden.
//!
//! D-12 — Post-Close-Edits: The OpenAPI doc on `export_attendance` documents
//! explicitly that nachtraegliche Anwesenheits-Korrekturen sich in jedem
//! nachfolgenden Export widerspiegeln. Der Service-Funnel liest in einer
//! frischen Transaction direkt aus `AttendanceDao::list_members_for_assembly`
//! — kein Cache, kein Snapshot des Export-Zeitpunkts.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
    Extension, Router,
};
use genossi_service::attendance_export::{AttendanceExportService, ExportFormat, ExportInclude};
use genossi_service::ServiceError;
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

/// State accessor trait for the Attendance-Export REST handlers.
///
/// The production `RestStateImpl` (in `genossi_bin`) implements this trait to
/// inject the `AttendanceExportServiceImpl` constructed in `RestStateImpl::new()`.
pub trait AttendanceExportRestState: Clone + Send + Sync + 'static {
    type AttendanceExportService: AttendanceExportService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;
    fn attendance_export_service(&self) -> Arc<Self::AttendanceExportService>;
}

/// D-13: PermissionDenied -> 403 Forbidden (statt 401 Unauthorized).
/// Spiegelt das Attendance-Pattern (D-26) — Frontend kann "kein Admin" (403)
/// von "Session ungueltig" (401) trennen. Alle anderen ServiceError-Varianten
/// delegieren an das globale `From<ServiceError>` in `genossi_rest/src/lib.rs`.
fn map_export_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}

/// Query parameters for `GET /api/assembly/{aid}/attendance-export/{format}`.
///
/// D-09: `?include=` defaults to `all` if the parameter is missing.
#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct ExportQuery {
    #[serde(default)]
    pub include: ExportIncludeQuery,
}

/// REST-layer mirror of `genossi_service::attendance_export::ExportInclude`.
///
/// Kept as a separate type so the REST schema can be derived via
/// `#[derive(Deserialize, ToSchema)]` without leaking utoipa into the
/// service-domain crate.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportIncludeQuery {
    #[default]
    All,
    Present,
}

impl From<ExportIncludeQuery> for ExportInclude {
    fn from(q: ExportIncludeQuery) -> ExportInclude {
        match q {
            ExportIncludeQuery::All => ExportInclude::All,
            ExportIncludeQuery::Present => ExportInclude::Present,
        }
    }
}

/// GET /api/assembly/{assembly_id}/attendance-export/{format}?include=all|present
///
/// D-12: Post-close attendance corrections take effect — der Export liest in
/// einer frischen Transaction direkt aus der `attendance`-Tabelle. Ein
/// erneuter Aufruf des Endpoints nach einer Korrektur liefert die aktualisierte
/// Liste. Es gibt keinen "Export-Snapshot" und keinen Cache, der das verhindern
/// koennte.
#[utoipa::path(
    get,
    tag = "AttendanceExport",
    path = "/api/assembly/{assembly_id}/attendance-export/{format}",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID (must be in status Closed; D-11)"),
        ("format" = String, Path, description = "csv | pdf | xlsx (D-14)"),
        ExportQuery,
    ),
    responses(
        (status = 200, description = "Export file (binary). D-12: reflects current attendance state — re-export after post-close edits to pick up corrections.", content_type = "application/octet-stream"),
        (status = 400, description = "Unknown format suffix (D-14) or invalid include query"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — not admin (D-13)"),
        (status = 404, description = "Assembly not found"),
        (status = 409, description = "Assembly is not in status Closed (D-11)"),
    ),
)]
#[instrument(skip(rest_state))]
pub async fn export_attendance<RestState: RestStateDef + AttendanceExportRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, format_str)): Path<(Uuid, String)>,
    Query(query): Query<ExportQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;

            // D-14: explicit whitelist on the format-suffix. Any other value
            // returns 400 BadRequest BEFORE we touch the service.
            let format = match format_str.as_str() {
                "csv" => ExportFormat::Csv,
                "pdf" => ExportFormat::Pdf,
                "xlsx" => ExportFormat::Xlsx,
                other => {
                    return Err(RestError::BadRequest(format!(
                        "unknown export format: {}",
                        other
                    )))
                }
            };
            let include: ExportInclude = query.include.into();

            let export = rest_state
                .attendance_export_service()
                .export(assembly_id, format, include, auth)
                .await
                .map_err(map_export_error)?;

            // D-15: filename comes from the service bundle; D-16: content-type
            // comes from the service bundle. Both are server-generated — no
            // user input flows into the Content-Disposition header.
            let cd = crate::http_util::content_disposition_attachment(&export.filename);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", export.content_type)
                .header("Content-Disposition", &cd)
                .body(Body::from(export.bytes))
                .unwrap())
        })
        .await,
    )
}

/// Router for `/api/assembly/{assembly_id}/attendance-export/{format}` (D-14).
///
/// Mounted at `/api/assembly` in `lib.rs::create_app` so the final route is
/// `/api/assembly/{assembly_id}/attendance-export/{format}`.
pub fn generate_export_route<RestState: RestStateDef + AttendanceExportRestState>(
) -> Router<RestState> {
    Router::new().route(
        "/{assembly_id}/attendance-export/{format}",
        get(export_attendance::<RestState>),
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(export_attendance),
    components(schemas(ExportQuery, ExportIncludeQuery)),
    tags((name = "AttendanceExport",
          description = "Teilnehmerlisten-Export fuer geschlossene Generalversammlungen (D-01..D-18). D-12: Post-Close-Anwesenheits-Korrekturen wirken sich auf jeden nachfolgenden Export aus — Re-Export nach Korrektur liefert die aktualisierte Liste."))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_export_error_permission_denied_returns_forbidden() {
        // D-13: PermissionDenied muss zu 403 Forbidden mappen, NICHT zu 401
        // Unauthorized. Frontend kann so "kein Admin" von "Session ungueltig"
        // unterscheiden.
        let mapped = map_export_error(ServiceError::PermissionDenied);
        assert!(
            matches!(mapped, RestError::Forbidden(_)),
            "PermissionDenied must map to Forbidden(403) for export endpoint"
        );
    }

    #[test]
    fn test_map_export_error_entity_not_found_delegates_to_global() {
        // Andere Varianten delegieren ans globale From<ServiceError>.
        let mapped = map_export_error(ServiceError::EntityNotFound(Uuid::nil()));
        assert!(
            matches!(mapped, RestError::NotFound),
            "EntityNotFound must map to RestError::NotFound via global From"
        );
    }

    #[test]
    fn test_map_export_error_conflict_delegates_to_global() {
        // D-11: Conflict("assembly_not_closed") muss zu 409 Conflict mappen
        // (via globalem From) — fuer die "Assembly ist nicht Closed"-Faelle.
        let mapped = map_export_error(ServiceError::Conflict(std::sync::Arc::from(
            "assembly_not_closed",
        )));
        assert!(
            matches!(mapped, RestError::Conflict(msg) if msg == "assembly_not_closed"),
            "Conflict must map to RestError::Conflict via global From"
        );
    }

    #[test]
    fn test_export_query_default_include_is_all() {
        // D-09: Default ist All.
        let q = ExportQuery::default();
        assert!(matches!(q.include, ExportIncludeQuery::All));
    }

    #[test]
    fn test_export_query_deserializes_include_all() {
        let json = serde_json::json!({ "include": "all" });
        let parsed: ExportQuery = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed.include, ExportIncludeQuery::All));
    }

    #[test]
    fn test_export_query_deserializes_include_present() {
        let json = serde_json::json!({ "include": "present" });
        let parsed: ExportQuery = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed.include, ExportIncludeQuery::Present));
    }

    #[test]
    fn test_export_query_empty_object_defaults_to_all() {
        // serde_json::from_value({}) must yield default — D-09 contract that
        // a missing ?include= param defaults to All.
        let json = serde_json::json!({});
        let parsed: ExportQuery = serde_json::from_value(json).unwrap();
        assert!(matches!(parsed.include, ExportIncludeQuery::All));
    }

    #[test]
    fn test_export_include_query_to_service_enum_mapping() {
        // From<ExportIncludeQuery> for ExportInclude muss 1:1 mappen.
        let all: ExportInclude = ExportIncludeQuery::All.into();
        let present: ExportInclude = ExportIncludeQuery::Present.into();
        assert_eq!(all, ExportInclude::All);
        assert_eq!(present, ExportInclude::Present);
    }
}
