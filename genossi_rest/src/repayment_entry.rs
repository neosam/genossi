//! REST handlers for the RepaymentEntry aggregate (Phase 8 Plan 05).
//!
//! Mirrors `genossi_rest/src/repayment_phase.rs` (1:1-Vorlage) with the
//! following Phase-8-spezifische Anpassungen:
//!
//! - **D-09 / D-12: Flat REST-Pfade** — `/api/repayment-entry/{id}` ohne
//!   Sub-Pfad-Nesting; `phase_id` als Create-Body-Feld und Listing-Query-Param.
//! - **D-10: Listing nur mit `?phase_id=<uuid>`** — keine status-/member-Filter.
//! - **D-12: PUT-Body** ist Optional-Field (`share_count_to_pay_out?`, `status?`)
//!   plus pflicht-`version`.
//! - **D-07/D-08: Batch-Endpoint** `POST /api/repayment-entry/batch-status`
//!   muss VOR `/{id}` im Router stehen (T-08-05-02 Mitigation), sonst frisst
//!   Axum's `:id`-Match das Literal "batch-status" als invalide Uuid.
//! - **W-05: Structured 409-Body** — der Service-Layer (Plan 03) liefert
//!   bereits `BatchFailureResponse`-strukturiertes JSON in `ServiceError::
//!   Conflict(Arc<str>)`; die globale `From<ServiceError> for RestError`-
//!   Konversion mapped das auf `RestError::Conflict(s)` → HTTP 409 mit Body.
//!
//! Error-Mapping: das globale `From<ServiceError> for RestError` in
//! `genossi_rest/src/lib.rs:97-113` deckt alle Fälle ab — KEIN lokaler
//! `map_*_error`-Override.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_rest_types::{
    BatchFailureResponse, BatchStatusRequest, CloseConflictResponse,
    CreateRepaymentEntryRequest, RepaymentEntryStatusTO, RepaymentEntryTO,
    UpdateRepaymentEntryRequest,
};
use genossi_service::repayment_entry::{
    RepaymentEntryBatchStatusInput, RepaymentEntryService, RepaymentEntrySubmission,
    RepaymentEntryUpdate,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestStateDef};

pub trait RepaymentEntryRestState: Clone + Send + Sync + 'static {
    type RepaymentEntryService: RepaymentEntryService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;

    fn repayment_entry_service(&self) -> Arc<Self::RepaymentEntryService>;
}

/// Listing-Query-Parameter für `GET /api/repayment-entry`.
///
/// D-10: nur `phase_id` als Filter; weitere Filter (status, member_id,
/// include_deleted) sind explizit deferred (Frontend macht client-side).
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListEntriesQuery {
    pub phase_id: Uuid,
}

// --- Handlers ---

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "",
    request_body = CreateRepaymentEntryRequest,
    responses(
        (status = 201, description = "Created", body = RepaymentEntryTO),
        (status = 400, description = "Validation Error (D-11.3 range)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member or Phase not found"),
        (status = 409, description = "Phase not Open (D-11.1)"),
    ),
)]
pub async fn create_repayment_entry<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<CreateRepaymentEntryRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let submission = RepaymentEntrySubmission {
                phase_id: body.phase_id,
                member_id: body.member_id,
                share_count_to_pay_out: body.share_count_to_pay_out,
            };
            let entry = rest_state
                .repayment_entry_service()
                .create_repayment_entry(&submission, auth)
                .await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "RepaymentEntries",
    path = "",
    params(ListEntriesQuery),
    responses(
        (status = 200, description = "List repayment entries for a phase", body = [RepaymentEntryTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_repayment_entries<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(q): Query<ListEntriesQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let entries = rest_state
                .repayment_entry_service()
                .list_repayment_entries_by_phase(q.phase_id, auth)
                .await?;
            let to_list: Vec<RepaymentEntryTO> =
                entries.iter().map(RepaymentEntryTO::from).collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to_list)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "RepaymentEntries",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "RepaymentEntry ID")),
    responses(
        (status = 200, description = "RepaymentEntry detail", body = RepaymentEntryTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
)]
pub async fn get_repayment_entry<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let entry = rest_state
                .repayment_entry_service()
                .get_repayment_entry(id, auth)
                .await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "RepaymentEntries",
    path = "/{id}",
    request_body = UpdateRepaymentEntryRequest,
    params(("id" = Uuid, Path, description = "RepaymentEntry ID")),
    responses(
        (status = 200, description = "Updated", body = RepaymentEntryTO),
        (status = 400, description = "Validation Error (D-11.3 range)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (PaidOut source/target D-05, version mismatch, or share_count edit on PaidOut ENTR-04)"),
    ),
)]
pub async fn update_repayment_entry<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRepaymentEntryRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let update = RepaymentEntryUpdate {
                share_count_to_pay_out: body.share_count_to_pay_out,
                status: body.status.as_ref().map(|s| s.into()),
                version: body.version,
            };
            let entry = rest_state
                .repayment_entry_service()
                .update_repayment_entry(id, &update, auth)
                .await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "RepaymentEntries",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "RepaymentEntry ID")),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (entry is PaidOut, ENTR-05)"),
    ),
)]
pub async fn delete_repayment_entry<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .repayment_entry_service()
                .delete_repayment_entry(id, auth)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "/batch-status",
    request_body = BatchStatusRequest,
    responses(
        (status = 200, description = "All entries toggled successfully", body = [RepaymentEntryTO]),
        (status = 400, description = "Validation Error (PaidOut as target_status, D-07)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found — at least one entry_id in the batch is missing or soft-deleted. The entire transaction is rolled back per D-08 (all-or-nothing). Aggregate-consistent with get/update/delete on /api/repayment-entry/{id}. The response body is the standard NotFound payload (NOT BatchFailureResponse)."),
        (status = 409, description = "Conflict — first failing entry rolled back transaction (D-08). Body matches BatchFailureResponse schema. Used for domain-level conflicts ONLY (e.g. source status is 'PaidOut'); for missing/soft-deleted entries see 404.", body = BatchFailureResponse),
    ),
)]
pub async fn batch_toggle_status<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<BatchStatusRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let input = RepaymentEntryBatchStatusInput {
                entry_ids: body.entry_ids.into(),
                target_status: (&body.target_status).into(),
            };
            let updated = rest_state
                .repayment_entry_service()
                .batch_toggle_status(&input, auth)
                .await?;
            let to_list: Vec<RepaymentEntryTO> =
                updated.iter().map(RepaymentEntryTO::from).collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to_list)?))
                .unwrap())
        })
        .await,
    )
}

// Phase 9 (PAYO-01 / D-07): Action-Endpoint fuer atomare Auszahlungs-Cascade.
// Pattern-Anker: open_repayment_phase (Phase 7) — kein Request-Body, kein
// Version-Body-Feld; Concurrency-Defense laeuft ueber Entry-Status-Guard +
// Version-Check im DAO-UPDATE (siehe 09-RESEARCH Frage 1). Single-only:
// KEINE Batch-Variante (Cascade ist sicherheitskritisch/irreversibel,
// Confirm-Dialog UI-05 ist pro Eintrag konzipiert; Batch deferred zu Phase 12).
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "/{id}/mark-paid-out",
    params(("id" = Uuid, Path, description = "RepaymentEntry ID")),
    responses(
        (status = 200,
         description = "Entry marked as PaidOut. Cascade: MemberAction::Verkauf created with \
                       shares_change=-N (where N=share_count_to_pay_out), Member.current_shares \
                       reduced by N, Member.action_count incremented by 1. All three writes commit \
                       in a single SQLite transaction with shared audit-process \
                       'repayment-entry.mark-paid-out'. Final per PAYO-04 (no toggle-back).",
         body = RepaymentEntryTO),
        (status = 400,
         description = "Validation Error (PAYO-03): Member.current_shares < entry.share_count_to_pay_out. \
                       Response body lists field='share_count_to_pay_out' with both values."),
        (status = 401, description = "Unauthorized (missing or invalid admin auth)"),
        (status = 404, description = "Entry not found or soft-deleted (checked before any write)"),
        (status = 409,
         description = "Conflict: entry status is not Open/Contacted (PAYO-04 — PaidOut is final), \
                       OR phase status is not Open (Defense-in-Depth), \
                       OR concurrent race produced version-mismatch on the entry update \
                       (loser of tokio::join! race per SC #5)."),
        (status = 500,
         description = "Internal consistency error: Re-Read after audited_update! returned None \
                       (Phase-8 BL-01 pattern — same-Tx invariant broken; should never happen \
                       in correctly-functioning DAO layer)."),
    ),
)]
pub async fn mark_paid_out<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let entry = rest_state
                .repayment_entry_service()
                .mark_paid_out(id, auth)
                .await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

/// Generate the Axum router for `/api/repayment-entry`.
///
/// **WICHTIG (T-08-05-02):** `/batch-status` MUSS VOR `/{id}` deklariert
/// werden — sonst frisst Axum's `/{id}`-Match das Literal "batch-status"
/// als Uuid und gibt 400 statt 200 zurück. Axum's router matcht in
/// Deklarations-Reihenfolge; das hier ist die einzige korrekte Anordnung.
pub fn generate_route<RestState: RestStateDef + RepaymentEntryRestState>() -> Router<RestState> {
    Router::new()
        // batch-status MUST be declared BEFORE /{id} to avoid Uuid-parse-collision.
        .route("/batch-status", post(batch_toggle_status::<RestState>))
        .route(
            "/",
            get(list_repayment_entries::<RestState>).post(create_repayment_entry::<RestState>),
        )
        .route(
            "/{id}",
            get(get_repayment_entry::<RestState>)
                .put(update_repayment_entry::<RestState>)
                .delete(delete_repayment_entry::<RestState>),
        )
        // Phase 9 (PAYO-01 / D-07): Single-only Action-Endpoint fuer atomare
        // Auszahlungs-Cascade. KEINE Batch-Variante (deferred zu Phase 12 —
        // Cascade ist sicherheitskritisch/irreversibel + UI-05 Confirm-Dialog
        // ist pro Eintrag konzipiert).
        // Reihenfolge ist hier egal (Axum matcht nach Pfad-Spezifitaet —
        // `/{id}/mark-paid-out` ist spezifischer als `/{id}`), aber Konvention
        // setzt Action-Endpoints ans Ende. Vorbild: repayment_phase.rs::generate_route
        // mit `.route("/{id}/open", ...)` / `.route("/{id}/close", ...)`.
        .route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_repayment_entries,
        create_repayment_entry,
        get_repayment_entry,
        update_repayment_entry,
        delete_repayment_entry,
        batch_toggle_status,
        // Phase 9 (PAYO-01):
        mark_paid_out,
    ),
    components(schemas(
        RepaymentEntryTO,
        RepaymentEntryStatusTO,
        CreateRepaymentEntryRequest,
        UpdateRepaymentEntryRequest,
        BatchStatusRequest,
        CloseConflictResponse,
        BatchFailureResponse,
    ))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apidoc_compiles() {
        // Compile-only test: if any handler is renamed without updating
        // the `paths(...)` list in ApiDoc, this would fail to compile.
        let _ = ApiDoc::openapi();
    }

    #[test]
    fn test_create_request_deserializes() {
        let json = format!(
            r#"{{"phase_id":"{}","member_id":"{}","share_count_to_pay_out":3}}"#,
            Uuid::new_v4(),
            Uuid::new_v4()
        );
        let req: CreateRepaymentEntryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.share_count_to_pay_out, 3);
    }

    #[test]
    fn test_list_query_deserializes_from_json() {
        // ListEntriesQuery serde-Roundtrip — verifies the field is named
        // exactly `phase_id` so axum's Query<ListEntriesQuery> extractor
        // can match `?phase_id=<uuid>`.
        let phase_id = Uuid::new_v4();
        let json = format!(r#"{{"phase_id":"{}"}}"#, phase_id);
        let q: ListEntriesQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q.phase_id, phase_id);
    }
}
