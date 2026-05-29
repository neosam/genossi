//! REST handlers for the RepaymentPhase aggregate (Phase 7 Plan 04).
//!
//! Mirrors `genossi_rest/src/assembly.rs` (1:1-Vorlage) plus the DELETE
//! handler pattern from `genossi_rest/src/member.rs` (assembly has no DELETE).
//!
//! Decisions enforced here:
//! - **D-02:** Lifecycle-Transitionen ausschließlich über die dedizierten
//!   Endpoints `POST /{id}/open` und `POST /{id}/close`; `PUT /{id}` hat
//!   KEIN `status`-Feld im Body (siehe `UpdateRepaymentPhaseRequest`).
//! - **D-03:** Open/Close-Endpoints akzeptieren KEIN Request-Body und
//!   prüfen KEIN `version`-Feld — Status-Guard ist die Concurrency-Defense.
//! - **D-14:** Pfad ist `/api/repayment-phase` (Singular).
//!
//! Error-Mapping: das globale `From<ServiceError> for RestError` in
//! `genossi_rest/src/lib.rs:97-113` deckt alle Fälle ab — KEIN lokaler
//! `map_*_error`-Override nötig (Phase 7 hat keinen 403-Bedarf).
//!
//! Field-Level-Validation (D-11/D-12) lebt im Service-Layer (Plan 03);
//! die REST-Validatoren sind hier rein strukturelle Compile-Konsistenz.

use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_rest_types::{
    CreateRepaymentPhaseRequest, RepaymentPhaseStatusTO, RepaymentPhaseTO,
    UpdateRepaymentPhaseRequest,
};
use genossi_service::repayment_phase::{
    RepaymentPhaseService, RepaymentPhaseSubmission, RepaymentPhaseUpdate,
};
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait RepaymentPhaseRestState: Clone + Send + Sync + 'static {
    type RepaymentPhaseService: RepaymentPhaseService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;

    fn repayment_phase_service(&self) -> Arc<Self::RepaymentPhaseService>;
}

// --- Validation helpers ---
//
// Field-Range-Validation (`fiscal_year in 2000..=2100`, `share_value > 0`)
// passiert auf der Service-Schicht (D-11/D-12, Plan 03 — siehe
// `genossi_service_impl::repayment_phase::validate_phase_fields`).
//
// Hier verifizieren wir nur die strukturelle Pflicht — die ist durch
// serde-Deserialisierung schon garantiert. Diese Helper existieren als
// pattern-konsistenter Anker zu `assembly.rs::validate_create_assembly_request`
// und damit zukünftige strukturelle Pflichtfeld-Checks einen klaren Platz haben.

pub fn validate_create_repayment_phase_request(
    _body: &CreateRepaymentPhaseRequest,
) -> Result<(), Vec<String>> {
    // Strukturelle Pflicht: serde-Deserialisierung erzwingt fiscal_year + share_value.
    // Range-Checks (D-11/D-12) liegen im Service-Layer.
    Ok(())
}

pub fn validate_update_repayment_phase_request(
    _body: &UpdateRepaymentPhaseRequest,
) -> Result<(), Vec<String>> {
    // Strukturelle Pflicht: serde-Deserialisierung erzwingt fiscal_year + share_value + version.
    // Range-Checks (D-11/D-12) liegen im Service-Layer.
    Ok(())
}

// --- Handlers ---

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "RepaymentPhases",
    path = "",
    responses(
        (status = 200, description = "List repayment phases", body = [RepaymentPhaseTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_repayment_phases<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let phases = rest_state
                .repayment_phase_service()
                .get_all_repayment_phases(auth)
                .await?;
            let to_list: Vec<RepaymentPhaseTO> = phases.iter().map(RepaymentPhaseTO::from).collect();
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
    post,
    tag = "RepaymentPhases",
    path = "",
    request_body = CreateRepaymentPhaseRequest,
    responses(
        (status = 201, description = "Created", body = RepaymentPhaseTO),
        (status = 400, description = "Validation Error (D-11/D-12)"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn create_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<CreateRepaymentPhaseRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            validate_create_repayment_phase_request(&body)
                .map_err(|errs| RestError::BadRequest(format!("Validation failed: {:?}", errs)))?;
            let submission = RepaymentPhaseSubmission {
                fiscal_year: body.fiscal_year,
                share_value: body.share_value,
            };
            let phase = rest_state
                .repayment_phase_service()
                .create_repayment_phase(&submission, auth)
                .await?;
            let to = RepaymentPhaseTO::from(&phase);
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
    tag = "RepaymentPhases",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    responses(
        (status = 200, description = "RepaymentPhase detail", body = RepaymentPhaseTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
)]
pub async fn get_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let phase = rest_state
                .repayment_phase_service()
                .get_repayment_phase(id, auth)
                .await?;
            let to = RepaymentPhaseTO::from(&phase);
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
    tag = "RepaymentPhases",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    request_body = UpdateRepaymentPhaseRequest,
    responses(
        (status = 200, description = "Updated", body = RepaymentPhaseTO),
        (status = 400, description = "Validation Error (D-11/D-12)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (Edit-Matrix violation D-04/D-07 or version mismatch)"),
    ),
)]
pub async fn update_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRepaymentPhaseRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            validate_update_repayment_phase_request(&body)
                .map_err(|errs| RestError::BadRequest(format!("Validation failed: {:?}", errs)))?;
            let update = RepaymentPhaseUpdate {
                fiscal_year: body.fiscal_year,
                share_value: body.share_value,
                version: body.version,
            };
            let phase = rest_state
                .repayment_phase_service()
                .update_repayment_phase(id, &update, auth)
                .await?;
            let to = RepaymentPhaseTO::from(&phase);
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
    post,
    tag = "RepaymentPhases",
    path = "/{id}/open",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    responses(
        (status = 200, description = "Opened", body = RepaymentPhaseTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation)"),
    ),
)]
pub async fn open_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let phase = rest_state
                .repayment_phase_service()
                .open_repayment_phase(id, auth)
                .await?;
            let to = RepaymentPhaseTO::from(&phase);
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
    post,
    tag = "RepaymentPhases",
    path = "/{id}/close",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    responses(
        (status = 200, description = "Closed", body = RepaymentPhaseTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Open)"),
    ),
)]
pub async fn close_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let phase = rest_state
                .repayment_phase_service()
                .close_repayment_phase(id, auth)
                .await?;
            let to = RepaymentPhaseTO::from(&phase);
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
    tag = "RepaymentPhases",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    responses(
        (status = 204, description = "RepaymentPhase deleted (soft-delete)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation, D-09)"),
    ),
)]
pub async fn delete_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .repayment_phase_service()
                .delete_repayment_phase(id, auth)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}

pub fn generate_route<RestState: RestStateDef + RepaymentPhaseRestState>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            get(list_repayment_phases::<RestState>).post(create_repayment_phase::<RestState>),
        )
        .route(
            "/{id}",
            get(get_repayment_phase::<RestState>)
                .put(update_repayment_phase::<RestState>)
                .delete(delete_repayment_phase::<RestState>),
        )
        .route("/{id}/open", post(open_repayment_phase::<RestState>))
        .route("/{id}/close", post(close_repayment_phase::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_repayment_phases,
        create_repayment_phase,
        get_repayment_phase,
        update_repayment_phase,
        delete_repayment_phase,
        open_repayment_phase,
        close_repayment_phase
    ),
    components(schemas(
        RepaymentPhaseTO,
        RepaymentPhaseStatusTO,
        CreateRepaymentPhaseRequest,
        UpdateRepaymentPhaseRequest
    ))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_create_request() -> CreateRepaymentPhaseRequest {
        CreateRepaymentPhaseRequest {
            fiscal_year: 2026,
            share_value: 12000,
        }
    }

    fn valid_update_request() -> UpdateRepaymentPhaseRequest {
        UpdateRepaymentPhaseRequest {
            fiscal_year: 2026,
            share_value: 13000,
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_validate_create_repayment_phase_request_ok() {
        // Structural validation passes for a well-formed body. Range-checks
        // are Service-Layer concern (D-11/D-12), not validated here.
        assert!(validate_create_repayment_phase_request(&valid_create_request()).is_ok());
    }

    #[test]
    fn test_validate_update_repayment_phase_request_ok() {
        let req = valid_update_request();
        assert!(validate_update_repayment_phase_request(&req).is_ok());
        // Type-system enforces a non-optional `version: Uuid` on
        // UpdateRepaymentPhaseRequest — verify the fixture passes a real UUID.
        assert_ne!(req.version, Uuid::nil(), "version must not be nil");
    }

    #[test]
    fn test_apidoc_compiles() {
        // Compile-only test: if any handler is renamed without updating
        // the `paths(...)` list in ApiDoc, this would fail to compile.
        let _ = ApiDoc::openapi();
    }
}
