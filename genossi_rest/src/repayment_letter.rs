//! Phase 13 D-13-02/03/04: REST-Handler fuer
//! `POST /api/repayment-phase/{phase_id}/letters/generate`.
//!
//! Direct-Download-Pattern aus Phase 11 (`repayment_export.rs`) 1:1.
//! Antwortet mit `application/pdf` + `Content-Disposition: attachment` +
//! `X-Document-Count: N` (D-13-04 — Anzahl Briefe nach Aggregation, dient dem
//! Frontend zur Toast-Pluralisierung, weil `entry_ids.len()` nach Aggregation
//! der falsche Zaehler waere).
//!
//! Lokales `map_letter_error` mappt `ServiceError::PermissionDenied` -> 403
//! (Phase 11 D-11 Pattern: Frontend kann "kein Admin" von "Session ungueltig"
//! differenzieren).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use genossi_service::repayment_letter::{
    RepaymentLetterBundle, RepaymentLetterDownload, RepaymentLetterDownloadFormat,
    RepaymentLetterService,
};
use genossi_service::ServiceError;

use crate::{error_handler, extract_auth_context, http_util, Context, RestError, RestStateDef};

/// Phase 11 D-11 / Phase 6 D-13: PermissionDenied -> Forbidden(403).
///
/// Das globale `From<ServiceError> for RestError` (in `lib.rs`) mappt
/// `PermissionDenied` auf `Unauthorized(401)`. Fuer Brief-Endpunkte wollen wir
/// aber zwischen "Session ungueltig" (401) und "Auth gueltig, aber kein Admin"
/// (403) unterscheiden — daher dieser lokale Override.
fn map_letter_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}

/// Request-Body fuer Bulk-Brief-Generierung.
///
/// D-13-03: flache Liste von `repayment_entry_id`s. Der Server gruppiert
/// serverseitig per `member_id` (D-13-04 Aggregation). Alle entry_ids MUESSEN
/// zur `phase_id` im URL-Pfad gehoeren — sonst 400 BadRequest mit
/// `entry_phase_mismatch`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateLettersRequest {
    /// IDs der RepaymentEntries fuer die Briefe generiert werden sollen.
    pub entry_ids: Vec<Uuid>,
}

/// Quick 260602-sgp: Query-Params fuer `GET /letters/download`.
///
/// `format` MUSS einer von `"zip"` oder `"pdf"` sein — alles andere liefert
/// 400 BadRequest mit Klartext-Hinweis.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DownloadQuery {
    /// Format: `"zip"` (Archiv mit Einzel-PDFs) oder `"pdf"` (gemerged Bundle-PDF).
    pub format: String,
}

/// State-Accessor-Trait fuer den Repayment-Letter-REST-Handler.
///
/// Die Production-`RestStateImpl` (in `genossi_bin`) implementiert dieses Trait
/// in Plan 13-05 Task 3, um den `RepaymentLetterServiceImpl` zu injizieren.
pub trait RepaymentLetterRestState: Clone + Send + Sync + 'static {
    type RepaymentLetterService: RepaymentLetterService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;
    fn repayment_letter_service(&self) -> Arc<Self::RepaymentLetterService>;
}

/// POST /api/repayment-phase/{phase_id}/letters/generate
///
/// Generiert Briefe fuer multi-selektierte RepaymentEntries und liefert ein
/// gebuendeltes Druck-PDF (D-13-01). Pro betroffenem Member entsteht zusaetzlich
/// ein persistiertes auditiertes MemberDocument.
///
/// Response-Header:
/// - `Content-Type: application/pdf`
/// - `Content-Disposition: attachment; filename="..."`
/// - `X-Document-Count: N` — Anzahl der unique Members nach Aggregation
///   (D-13-04). Frontend nutzt diesen Wert fuer die Pluralisierung des
///   Erfolgs-Toasts, weil `entry_ids.len()` nach Aggregation falsch waere.
#[utoipa::path(
    post,
    path = "/api/repayment-phase/{phase_id}/letters/generate",
    params(
        ("phase_id" = Uuid, Path, description = "RepaymentPhase UUID — phase muss Open oder Closed sein"),
    ),
    request_body = GenerateLettersRequest,
    responses(
        (status = 200,
            description = "Bundle-PDF aller Anschreiben — Content-Type application/pdf, Content-Disposition attachment, X-Document-Count: N (Anzahl Briefe nach Aggregation)",
            content_type = "application/pdf"),
        (status = 400, description = "Validation-Fehler: entry_ids leer, entry_phase_mismatch, oder unknown entry_ids"),
        (status = 401, description = "Session ungueltig oder fehlt"),
        (status = 403, description = "Auth gueltig, aber kein Vorstand (Helfer-Auth)"),
        (status = 404, description = "RepaymentPhase mit dieser ID nicht gefunden"),
        (status = 409, description = "RepaymentPhase im Preparation-Status — phase_not_active"),
    ),
    tag = "RepaymentLetter"
)]
#[instrument(skip(rest_state))]
pub async fn generate_letters<RestState: RestStateDef + RepaymentLetterRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(phase_id): Path<Uuid>,
    Json(body): Json<GenerateLettersRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = extract_auth_context(Some(context))?;

            if body.entry_ids.is_empty() {
                return Err(RestError::BadRequest(
                    "entry_ids must not be empty".to_string(),
                ));
            }

            let entry_ids: Arc<[Uuid]> = body.entry_ids.into();

            let result: RepaymentLetterBundle = rest_state
                .repayment_letter_service()
                .generate(phase_id, entry_ids, auth)
                .await
                .map_err(map_letter_error)?;

            let cd = http_util::content_disposition_attachment(&result.filename);

            // D-13-04: X-Document-Count = Anzahl der unique Members nach
            // Aggregation. Frontend (Plan 06) liest diesen Header fuer die
            // Toast-Pluralisierung, weil entry_ids.len() nach Aggregation
            // der falsche Zaehler waere.
            let document_count = result.document_ids.len();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/pdf")
                .header("Content-Disposition", &cd)
                .header("X-Document-Count", document_count.to_string())
                .body(Body::from(result.bundle_bytes))
                .unwrap())
        })
        .await,
    )
}

/// Quick 260602-sgp: `GET /api/repayment-phase/{phase_id}/letters/download?format=zip|pdf`
///
/// Bulk-Download aller bereits persistierten RepaymentLetter-PDFs der Phase.
/// NICHT-Neu-Render: Service laedt MemberDocuments mit
/// `DocumentType::RepaymentLetter` aus dem Document-Storage. Fehlende Files
/// werden geskippt; Count im `X-Skipped-Count`-Header.
///
/// Response-Header:
/// - `Content-Type: application/zip` ODER `application/pdf`
/// - `Content-Disposition: attachment; filename="auszahlungs_anschreiben_GJ_{fy}.{zip|pdf}"`
/// - `X-Document-Count: N` — Anzahl erfolgreich zusammengefasster Letters
/// - `X-Skipped-Count: M` — Anzahl fehlender Files im Storage
#[utoipa::path(
    get,
    path = "/api/repayment-phase/{phase_id}/letters/download",
    params(
        ("phase_id" = Uuid, Path, description = "RepaymentPhase UUID — phase muss Open oder Closed sein"),
        DownloadQuery,
    ),
    responses(
        (status = 200,
            description = "Bulk-Download als ZIP oder Bundle-PDF",
            content_type = "application/octet-stream"),
        (status = 400, description = "Ungueltiges format (nur 'zip' und 'pdf' erlaubt)"),
        (status = 401, description = "Session ungueltig oder fehlt"),
        (status = 403, description = "Auth gueltig, aber kein Vorstand (Helfer-Auth)"),
        (status = 404, description = "Phase nicht gefunden ODER keine persistierten Letters"),
        (status = 409, description = "Phase im Preparation-Status — phase_not_active"),
    ),
    tag = "RepaymentLetter"
)]
#[instrument(skip(rest_state))]
pub async fn download_letters<RestState: RestStateDef + RepaymentLetterRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(phase_id): Path<Uuid>,
    Query(query): Query<DownloadQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = extract_auth_context(Some(context))?;

            let format = match query.format.as_str() {
                "zip" => RepaymentLetterDownloadFormat::Zip,
                "pdf" => RepaymentLetterDownloadFormat::Pdf,
                other => {
                    return Err(RestError::BadRequest(format!(
                        "invalid format '{}': use 'zip' or 'pdf'",
                        other
                    )));
                }
            };

            let result: RepaymentLetterDownload = rest_state
                .repayment_letter_service()
                .download_bundle(phase_id, format, auth)
                .await
                .map_err(map_letter_error)?;

            let cd = http_util::content_disposition_attachment(&result.filename);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", result.content_type)
                .header("Content-Disposition", &cd)
                .header("X-Document-Count", result.document_count.to_string())
                .header("X-Skipped-Count", result.skipped_count.to_string())
                .body(Body::from(result.bytes))
                .unwrap())
        })
        .await,
    )
}

/// Router-Generator. Mount via
/// `.nest("/api/repayment-phase", generate_letter_route::<RestState>())`.
///
/// Axum 0.8.3 erlaubt mehrere `.nest`-Aufrufe mit demselben Prefix, solange
/// die inneren Pfade-Segmente eindeutig sind. `/{phase_id}/letters/generate`
/// ist disjunkt von `/{phase_id}/export/{format}`.
pub fn generate_letter_route<RestState: RestStateDef + RepaymentLetterRestState>(
) -> Router<RestState> {
    Router::new()
        .route(
            "/{phase_id}/letters/generate",
            post(generate_letters::<RestState>),
        )
        .route(
            "/{phase_id}/letters/download",
            get(download_letters::<RestState>),
        )
}

#[derive(OpenApi)]
#[openapi(
    paths(generate_letters, download_letters),
    components(schemas(GenerateLettersRequest)),
    tags(
        (name = "RepaymentLetter",
         description = "Phase 13: Bulk-PDF-Anschreiben fuer Nicht-Email-Mitglieder. Vorstand-only.")
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_letter_error_permission_denied_to_403() {
        // D-11 / Phase 6 D-13: PermissionDenied -> Forbidden(403), NICHT Unauthorized(401).
        let err = map_letter_error(ServiceError::PermissionDenied);
        assert!(
            matches!(err, RestError::Forbidden(_)),
            "PermissionDenied muss zu Forbidden(403) mappen, nicht zu Unauthorized(401)"
        );
    }

    #[test]
    fn test_map_letter_error_entity_not_found_passthrough() {
        // EntityNotFound nutzt globales From -> NotFound(404).
        let err = map_letter_error(ServiceError::EntityNotFound(Uuid::new_v4()));
        assert!(matches!(err, RestError::NotFound));
    }

    #[test]
    fn test_map_letter_error_conflict_passthrough() {
        // Conflict("phase_not_active") -> Conflict(409) via globalem From.
        let err = map_letter_error(ServiceError::Conflict(std::sync::Arc::from(
            "phase_not_active",
        )));
        assert!(matches!(err, RestError::Conflict(_)));
    }

    #[test]
    fn test_generate_letters_request_deserialization() {
        let json = r#"{"entry_ids":["11111111-1111-1111-1111-111111111111"]}"#;
        let req: GenerateLettersRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.entry_ids.len(), 1);
    }

    #[test]
    fn test_generate_letters_request_empty_list_deserialization() {
        // Leere Liste deserialisiert sauber — Empty-Check passiert im Handler.
        let json = r#"{"entry_ids":[]}"#;
        let req: GenerateLettersRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.entry_ids.len(), 0);
    }

    // ── Quick 260602-sgp: DownloadQuery deserialization ───────────────

    #[test]
    fn test_download_query_zip() {
        let q: DownloadQuery = DownloadQuery {
            format: "zip".to_string(),
        };
        assert_eq!(q.format, "zip");
    }

    #[test]
    fn test_download_query_pdf() {
        let q: DownloadQuery = DownloadQuery {
            format: "pdf".to_string(),
        };
        assert_eq!(q.format, "pdf");
    }

    #[test]
    fn test_download_query_json_deserialization_zip() {
        // Verifiziert dass das serde-Mapping in Axum's Query-Extractor
        // einen `format=zip`-String akzeptiert (form-urlencoded).
        let q: DownloadQuery = serde_json::from_str(r#"{"format":"zip"}"#).unwrap();
        assert_eq!(q.format, "zip");
    }

    #[test]
    fn test_download_query_json_deserialization_pdf() {
        let q: DownloadQuery = serde_json::from_str(r#"{"format":"pdf"}"#).unwrap();
        assert_eq!(q.format, "pdf");
    }
}
