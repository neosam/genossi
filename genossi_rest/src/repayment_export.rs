//! Phase 11 (EXPO-01, EXPO-03, EXPO-05): REST-Handler fuer RepaymentExportService.
//!
//! Route (D-14): `GET /api/repayment-phase/{phase_id}/export/{format}?include=open|all|paid`
//!
//! Vorbild: genossi_rest/src/attendance_export.rs (Phase 6).
//! Anpassungen:
//!   - D-12: Format-Whitelist NUR `pdf` (csv/xlsx -> 400, Pitfall #3)
//!   - D-03: Default-Include = Open (Banking-Workflow "noch nicht ausbezahlt")
//!   - D-11: PermissionDenied -> 403 via lokalem map_export_error (Frontend kann
//!           "kein Admin" 403 von "Session ungueltig" 401 unterscheiden)

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
    Extension, Router,
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use genossi_service::repayment_export::{
    ExportFormat, ExportInclude, RepaymentExportService,
};
use genossi_service::ServiceError;

use crate::{error_handler, extract_auth_context, http_util, Context, RestError, RestStateDef};

/// D-11 / Phase 6 D-13: PermissionDenied -> Forbidden(403).
///
/// Das globale `From<ServiceError> for RestError` (in `lib.rs`) mappt
/// `PermissionDenied` auf `Unauthorized(401)`. Fuer Export-Endpunkte wollen wir
/// aber zwischen "Session ungueltig" (401) und "Auth gueltig, aber kein Admin"
/// (403) unterscheiden — daher dieser lokale Override.
fn map_export_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}

/// Query-Parameter fuer `GET /api/repayment-phase/{phase_id}/export/{format}`.
///
/// D-03: `?include=` defaultet auf `Open` (Banking-Workflow) wenn Parameter fehlt.
#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct ExportQuery {
    #[serde(default)]
    pub include: ExportIncludeQuery,
}

/// REST-Layer-Mirror von `genossi_service::repayment_export::ExportInclude`.
///
/// Eigener Type damit der REST-Schema-Layer per `derive(Deserialize, ToSchema)`
/// gebaut werden kann ohne utoipa in das Service-Domain-Crate zu leaken.
///
/// D-03: Default = `Open` (Banking-Vorlage-Use-Case "noch nicht ausbezahlt").
#[derive(Debug, Default, Deserialize, ToSchema, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ExportIncludeQuery {
    #[default]
    Open,
    All,
    Paid,
}

impl From<ExportIncludeQuery> for ExportInclude {
    fn from(q: ExportIncludeQuery) -> ExportInclude {
        match q {
            ExportIncludeQuery::Open => ExportInclude::Open,
            ExportIncludeQuery::All => ExportInclude::All,
            ExportIncludeQuery::Paid => ExportInclude::Paid,
        }
    }
}

/// State-Accessor-Trait fuer die Repayment-Export-REST-Handler.
///
/// Die Production-`RestStateImpl` (in `genossi_bin`) implementiert dieses Trait
/// in Plan 11.05, um den `RepaymentExportServiceImpl` zu injizieren.
pub trait RepaymentExportRestState: Clone + Send + Sync + 'static {
    type RepaymentExportService: RepaymentExportService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;
    fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService>;
}

/// GET /api/repayment-phase/{phase_id}/export/{format}?include=open|all|paid
///
/// D-12 / Pitfall #3: Format-Whitelist hat NUR `pdf`. csv/xlsx/anderer Input
/// -> 400 BadRequest BEVOR der Service angesprochen wird.
#[utoipa::path(
    get,
    path = "/api/repayment-phase/{phase_id}/export/{format}",
    params(
        ("phase_id" = Uuid, Path, description = "RepaymentPhase UUID"),
        ("format" = String, Path, description = "Export-Format. NUR 'pdf' in Phase 11 (D-12)."),
        ExportQuery,
    ),
    responses(
        (status = 200, description = "PDF-Bytes der Auszahlungsliste",
            content_type = "application/pdf"),
        (status = 400, description = "Unbekanntes Format (z.B. ?format=csv) oder ungueltige Query-Params"),
        (status = 401, description = "Session ungueltig oder fehlt"),
        (status = 403, description = "Auth gueltig, aber kein Vorstand (admin-Privilege fehlt)"),
        (status = 404, description = "RepaymentPhase mit dieser ID nicht gefunden"),
        (status = 409, description = "RepaymentPhase im Status Preparation — nicht exportierbar"),
    ),
    tag = "RepaymentExport"
)]
#[instrument(skip(rest_state))]
pub async fn export_repayment<RestState: RestStateDef + RepaymentExportRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((phase_id, format_str)): Path<(Uuid, String)>,
    Query(query): Query<ExportQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = extract_auth_context(Some(context))?;

            // D-12 / Pitfall #3: Format-Whitelist NUR pdf. csv/xlsx/andere -> 400.
            let format = match format_str.as_str() {
                "pdf" => ExportFormat::Pdf,
                other => {
                    return Err(RestError::BadRequest(format!(
                        "unknown export format: {}",
                        other
                    )))
                }
            };

            let include: ExportInclude = query.include.into();

            let export = rest_state
                .repayment_export_service()
                .export(phase_id, format, include, auth)
                .await
                .map_err(map_export_error)?;

            let cd = http_util::content_disposition_attachment(&export.filename);

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

/// Router fuer `/{phase_id}/export/{format}` (gemounted unter `/api/repayment-phase`).
///
/// Axum 0.8.3 merged diesen Mount mit dem existierenden
/// `repayment_phase::generate_route()` unter dem gleichen Prefix — die Pfade
/// `/{phase_id}` und `/{phase_id}/export/{format}` kollidieren nicht.
pub fn generate_export_route<RestState: RestStateDef + RepaymentExportRestState>(
) -> Router<RestState> {
    Router::new().route(
        "/{phase_id}/export/{format}",
        get(export_repayment::<RestState>),
    )
}

#[derive(OpenApi)]
#[openapi(
    paths(export_repayment),
    components(schemas(ExportQuery, ExportIncludeQuery)),
    tags((name = "RepaymentExport",
          description = "Phase 11: PDF-Export der Auszahlungsliste fuer RepaymentPhase. Vorstand-only."))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_include_query_default_is_open() {
        // D-03: Default Open fuer Banking-Workflow.
        assert_eq!(ExportIncludeQuery::default(), ExportIncludeQuery::Open);
    }

    #[test]
    fn test_export_include_query_deserialization_lowercase() {
        // serde rename_all = "lowercase"
        let q: ExportIncludeQuery = serde_json::from_str("\"open\"").unwrap();
        assert_eq!(q, ExportIncludeQuery::Open);
        let q: ExportIncludeQuery = serde_json::from_str("\"all\"").unwrap();
        assert_eq!(q, ExportIncludeQuery::All);
        let q: ExportIncludeQuery = serde_json::from_str("\"paid\"").unwrap();
        assert_eq!(q, ExportIncludeQuery::Paid);
    }

    #[test]
    fn test_from_export_include_query_to_export_include() {
        assert!(matches!(
            ExportInclude::from(ExportIncludeQuery::Open),
            ExportInclude::Open
        ));
        assert!(matches!(
            ExportInclude::from(ExportIncludeQuery::All),
            ExportInclude::All
        ));
        assert!(matches!(
            ExportInclude::from(ExportIncludeQuery::Paid),
            ExportInclude::Paid
        ));
    }

    #[test]
    fn test_map_export_error_permission_denied_to_403() {
        // D-11 / Phase 6 D-13: PermissionDenied -> Forbidden(403), NICHT Unauthorized(401).
        let err = map_export_error(ServiceError::PermissionDenied);
        assert!(
            matches!(err, RestError::Forbidden(_)),
            "PermissionDenied muss zu Forbidden(403) mappen, nicht zu Unauthorized(401)"
        );
    }

    #[test]
    fn test_map_export_error_entity_not_found_passthrough() {
        // EntityNotFound nutzt globales From -> NotFound(404).
        let err = map_export_error(ServiceError::EntityNotFound(Uuid::new_v4()));
        assert!(matches!(err, RestError::NotFound));
    }

    #[test]
    fn test_map_export_error_conflict_passthrough() {
        // Conflict("phase_not_exportable") -> Conflict(409) via globalem From.
        let err = map_export_error(ServiceError::Conflict(std::sync::Arc::from(
            "phase_not_exportable",
        )));
        assert!(matches!(err, RestError::Conflict(_)));
    }

    #[test]
    fn test_export_query_default_via_serde() {
        // Empty JSON object -> ExportQuery { include: Open } (D-03).
        let q: ExportQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(q.include, ExportIncludeQuery::Open);
    }
}
