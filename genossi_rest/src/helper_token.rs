//! REST-Layer für das helper_token-Aggregat (Phase 2).
//!
//! Enthält:
//!   - `HelperTokenRestState`-Trait — exponiert den `HelperTokenService` an Handler
//!   - 3 Vorstand-Handler (admin, hinter Auth-Middleware): create/list/revoke
//!   - 1 Public-Handler (kein extract_auth_context): redeem mit Set-Cookie
//!   - `generate_route` (Vorstand) + `generate_public_route` (Public)
//!   - `ApiDoc` (Vorstand) + `PublicApiDoc` (Public)
//!
//! D-21: Vorstand-Endpoints sind admin-only (Permission-Check im Service-Layer).
//! D-22: Public-Redeem läuft ohne extract_auth_context; Auth-Middleware muss
//!       diesen Pfad whitelisten (`auth_middleware.rs::is_auth_excluded` Pattern).
//! D-24: ServiceError-Discriminator-Strings für 410/403-Mapping (siehe
//!       `redeem_helper_token`-Handler).

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_rest_types::{
    CreateHelperTokenRequest, HelperSessionTO, HelperTokenCreateResponseTO, HelperTokenStatusTO,
    HelperTokenTO, RedeemRequest, RedeemResponse,
};
use genossi_service::helper_token::{HelperToken, HelperTokenService, HelperTokenSubmission};
use genossi_service::session::SessionService;
use genossi_service::{ServiceError, ValidationFailureItem};
use genossi_service_impl::helper_token::render_qr_svg;

/// Build the QR-payload string from a plain-text code. Mirrors the format
/// used in `HelperTokenServiceImpl::create_helper_token` (Phase 2 Plan 02-05)
/// so re-displayed QR cards point at the same magic-link the original card
/// did. APP_URL has the same fallback semantics as the service-impl helper
/// (default to `http://localhost:3000/` for mock_auth / e2e-tests).
fn build_qr_payload(code: &str) -> String {
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string());
    format!("{}/helper?code={}", app_url.trim_end_matches('/'), code)
}

/// Build a `HelperTokenTO` from a service-domain `HelperToken`, attaching
/// the plain-text code AND a regenerated QR SVG when the row is post-migration.
/// Pre-update rows (code == None) surface as `code: None, qr_svg: None`;
/// the frontend renders a "revoke + recreate" hint for those.
///
/// Errors during QR rendering are surfaced as `RestError::InternalError` —
/// they should be impossible for a 10-char Crockford code, but a sentinel
/// path keeps the handler honest if the renderer ever returns Err.
fn helper_token_to_with_code(token: &HelperToken) -> Result<HelperTokenTO, RestError> {
    let status = if token.revoked_at.is_some() {
        HelperTokenStatusTO::Revoked
    } else if token.used_at.is_some() {
        HelperTokenStatusTO::Used
    } else {
        HelperTokenStatusTO::Open
    };
    let (code_opt, qr_svg_opt) = match token.code.as_deref() {
        Some(code) => {
            let svg = render_qr_svg(&build_qr_payload(code))
                .map_err(|e| RestError::InternalError(format!("qr regenerate failed: {:?}", e)))?;
            (Some(code.to_string()), Some(svg))
        }
        None => (None, None),
    };
    Ok(HelperTokenTO {
        id: token.id,
        assembly_id: token.assembly_id,
        memo: token.memo.to_string(),
        status,
        used_at: token.used_at,
        revoked_at: token.revoked_at,
        created: Some(token.created),
        version: token.version,
        code: code_opt,
        qr_svg: qr_svg_opt,
    })
}
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait HelperTokenRestState: Clone + Send + Sync + 'static {
    type HelperTokenService: HelperTokenService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;

    fn helper_token_service(&self) -> Arc<Self::HelperTokenService>;
}

/// Validates the body of POST /api/assembly/{aid}/helper-tokens (D-21).
/// `memo` is required, max 256 characters (Unicode scalar values, mirrors
/// `assembly.rs::validate_required_field`).
fn validate_create_helper_token_request(
    body: &CreateHelperTokenRequest,
) -> Result<(), Vec<ValidationFailureItem>> {
    let mut errors = Vec::new();
    let memo = body.memo.trim();
    if memo.is_empty() {
        errors.push(ValidationFailureItem {
            field: Arc::from("memo"),
            message: Arc::from("missing"),
        });
    } else if memo.chars().count() > 256 {
        errors.push(ValidationFailureItem {
            field: Arc::from("memo"),
            message: Arc::from("too_long (max 256)"),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ============================================================================
// Vorstand-Handler 1: create_helper_token (POST /, returns 201)
// ============================================================================

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Helper Tokens",
    path = "",
    params(("assembly_id" = Uuid, Path, description = "Assembly ID")),
    request_body = CreateHelperTokenRequest,
    responses(
        (status = 201, description = "Helper token created (one-time qr_svg + code in body)", body = HelperTokenCreateResponseTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Assembly not found"),
        (status = 409, description = "Conflict (assembly Closed)"),
        (status = 422, description = "Validation error"),
    ),
)]
pub async fn create_helper_token<RestState: RestStateDef + HelperTokenRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(assembly_id): Path<Uuid>,
    Json(body): Json<CreateHelperTokenRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            validate_create_helper_token_request(&body).map_err(|errs| {
                let messages: Vec<String> = errs
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect();
                RestError::BadRequest(messages.join(", "))
            })?;
            let submission = HelperTokenSubmission {
                memo: Arc::from(body.memo.trim()),
            };
            let created = rest_state
                .helper_token_service()
                .create_helper_token(assembly_id, &submission, auth)
                .await?;

            // ADR-2026-05-06: build the inner TO with code + regenerated QR
            // attached (the persistent path). The outer response also carries
            // `code` + `qr_svg` at the top level — that is the legacy single-
            // use display field; the inner TO is the always-available path
            // used by re-display via list_helper_tokens.
            let token_to = helper_token_to_with_code(&created.token)?;
            let response = HelperTokenCreateResponseTO {
                token: token_to,
                code: created.code.to_string(),
                qr_svg: created.qr_svg.to_string(),
            };

            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response)?))
                .unwrap())
        })
        .await,
    )
}

// ============================================================================
// Vorstand-Handler 2: list_helper_tokens (GET /, returns 200)
// ============================================================================

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Helper Tokens",
    path = "",
    params(("assembly_id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "List of helper tokens for this assembly", body = [HelperTokenTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_helper_tokens<RestState: RestStateDef + HelperTokenRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(assembly_id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let tokens = rest_state
                .helper_token_service()
                .list_for_assembly(assembly_id, auth)
                .await?;
            // ADR-2026-05-06: each TO carries the plain-text code (when
            // present) plus a freshly regenerated QR SVG. Legacy rows with
            // code = None surface as `code: None, qr_svg: None` so the
            // frontend can render the "revoke + recreate" hint.
            let to_list: Vec<HelperTokenTO> = tokens
                .iter()
                .map(helper_token_to_with_code)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to_list)?))
                .unwrap())
        })
        .await,
    )
}

// ============================================================================
// Vorstand-Handler 3: revoke_helper_token (POST /{token_id}/revoke, returns 200)
// ============================================================================

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Helper Tokens",
    path = "/{token_id}/revoke",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID"),
        ("token_id" = Uuid, Path, description = "Helper Token ID"),
    ),
    responses(
        (status = 200, description = "Token revoked", body = HelperTokenTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (token already used or assembly closed)"),
    ),
)]
pub async fn revoke_helper_token<RestState: RestStateDef + HelperTokenRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, token_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let token = rest_state
                .helper_token_service()
                .revoke_helper_token(assembly_id, token_id, auth)
                .await?;
            // ADR-2026-05-06: revoked tokens still carry their plain-text code
            // in the DB (the column is immutable). The frontend hides the
            // "QR/Code anzeigen" button for revoked rows by status, so this
            // is informational only — but consistent with the list response.
            let to = helper_token_to_with_code(&token)?;
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

// ============================================================================
// Public Handler 4: redeem_helper_token (POST /redeem, PUBLIC, Set-Cookie)
// ============================================================================

// skip(body): RedeemRequest enthält den Klartext-Redeem-Code (#[derive(Debug)]),
// der bei RUST_LOG=debug sonst im Tracing-Span landen würde — der Service-Layer
// vermeidet das bewusst, der Span darf es nicht aushebeln (replay-bare Codes).
#[instrument(skip(rest_state, body))]
#[utoipa::path(
    post,
    tag = "Helper Redeem",
    path = "/redeem",
    request_body = RedeemRequest,
    responses(
        (status = 200, description = "Redeem successful, session cookie set", body = RedeemResponse),
        (status = 400, description = "Invalid code format"),
        (status = 403, description = "Token revoked or assembly not Open"),
        (status = 404, description = "Token unknown"),
        (status = 410, description = "Token already redeemed"),
        (status = 429, description = "Rate limit exceeded"),
    ),
)]
pub async fn redeem_helper_token<RestState: RestStateDef + HelperTokenRestState>(
    rest_state: State<RestState>,
    Json(body): Json<RedeemRequest>,
) -> Response {
    error_handler(
        (async {
            // NO extract_auth_context — this is a PUBLIC endpoint (D-22).
            // Differential ServiceError-Mapping (D-24): we do an explicit match here
            // because the standard From<ServiceError> for RestError doesn't differentiate
            // the helper_token Conflict-payloads (already_used / revoked / assembly_not_open).
            let result = rest_state
                .helper_token_service()
                .redeem_helper_token(&body.code)
                .await;

            let success = match result {
                Ok(s) => s,
                Err(ServiceError::ValidationError(_)) => {
                    return Err(RestError::BadRequest("invalid_code_format".to_string()));
                }
                Err(ServiceError::EntityNotFound(_)) => {
                    return Err(RestError::NotFound);
                }
                Err(ServiceError::Conflict(payload)) => {
                    let p = payload.as_ref();
                    if p == "already_used" {
                        return Err(RestError::Gone("already_used".to_string()));
                    } else if p == "revoked" || p == "assembly_not_open" {
                        return Err(RestError::Forbidden(p.to_string()));
                    } else {
                        return Err(RestError::Conflict(p.to_string()));
                    }
                }
                Err(other) => return Err(other.into()),
            };

            // Build Set-Cookie header (D-22, D-18 max-age 86400):
            // app_session=<session_id>; Path=/; HttpOnly; SameSite=Strict; Secure; Max-Age=86400
            let cookie_value = format!(
                "app_session={}; Path=/; HttpOnly; SameSite=Strict; Secure; Max-Age=86400",
                success.session_id
            );

            // Format expires_at as ISO8601 string (Unix timestamp -> ISO).
            let expires_at_iso = time::OffsetDateTime::from_unix_timestamp(success.expires_at)
                .map_err(|e| RestError::InternalError(format!("invalid expires_at: {}", e)))?
                .format(&time::format_description::well_known::Iso8601::DEFAULT)
                .map_err(|e| RestError::InternalError(format!("format expires_at: {}", e)))?;

            let response_body = RedeemResponse {
                assembly_id: success.assembly_id,
                expires_at: expires_at_iso,
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .header(
                    header::SET_COOKIE,
                    HeaderValue::from_str(&cookie_value).map_err(|e| {
                        RestError::InternalError(format!("invalid cookie value: {}", e))
                    })?,
                )
                .body(Body::new(serde_json::to_string(&response_body)?))
                .unwrap())
        })
        .await,
    )
}

// ============================================================================
// Public Handler 5+6: Helper-Session-Lookup + Logout (Phase 4 D-06, D-07)
// ============================================================================

/// Read the `app_session` cookie value from the request headers.
/// Returns `None` if the cookie is absent, malformed, or empty.
///
/// Pattern mirrors the SET-side in `redeem_helper_token` (Set-Cookie at
/// `app_session=<sid>; Path=/; HttpOnly; SameSite=Strict; Secure;
/// Max-Age=86400`). RFC-6265 cookie strings are split on `;` and each
/// `name=value` pair is trimmed before matching against the
/// `app_session=` prefix. We deliberately do NOT use `tower_cookies::Cookies`
/// here to keep the handlers symmetric with `redeem_helper_token` — both the
/// SET-side and READ-side now operate directly on `header::COOKIE` /
/// `header::SET_COOKIE`. NO Extension<Context>: these endpoints are PUBLIC
/// (D-22 parallel) and their authentication is the cookie itself, validated
/// against the SessionService below.
fn read_session_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').map(str::trim).find_map(|kv| {
        let mut it = kv.splitn(2, '=');
        match (it.next(), it.next()) {
            (Some("app_session"), Some(v)) if !v.is_empty() => Some(v.to_string()),
            _ => None,
        }
    })
}

/// GET /api/helper/session — returns 200 + HelperSessionTO if a valid
/// Helper-Session-Cookie is present. Used by Frontend (Phase 4 D-06) for
/// Auto-Redirect on the `/helper`-mount.
///
/// T-04-01 Mitigation: response is exactly 3 keys (assembly_id, assembly_name,
/// expires_at) — no token-id, memo, or member PII.
/// T-04-02 Mitigation: rejects 401 if the `app_session` does not map to a
/// helper_token row (i.e. it is an admin/OIDC session, not a Helper one).
#[instrument(skip(rest_state, headers))]
#[utoipa::path(
    get,
    tag = "Helper Session",
    path = "/session",
    responses(
        (status = 200, description = "Active helper session", body = HelperSessionTO),
        (status = 401, description = "No helper session present"),
    ),
)]
pub async fn get_helper_session<RestState: RestStateDef + HelperTokenRestState>(
    rest_state: State<RestState>,
    headers: HeaderMap,
) -> Response {
    error_handler(
        (async {
            let session_id_str = read_session_cookie(&headers).ok_or(RestError::Unauthorized)?;

            // Step 1: validate the session via SessionService — must exist
            // (not expired, not invalidated). 401 on any error / unknown.
            let user_session = rest_state
                .session_service()
                .verify_user_session(&session_id_str)
                .await
                .map_err(|_| RestError::Unauthorized)?
                .ok_or(RestError::Unauthorized)?;

            // Step 2: confirm it is a Helper-Session (T-04-02). The reverse
            // lookup returns Some only if a non-deleted helper_token row
            // carries this session_id. Admin/OIDC sessions return None → 401.
            let info = rest_state
                .helper_token_service()
                .find_assembly_for_session(&session_id_str)
                .await
                .map_err(|_| RestError::Unauthorized)?
                .ok_or(RestError::Unauthorized)?;

            // Step 3: format expires_at as ISO8601 (Unix timestamp from the
            // session row → ISO-formatted string for the JSON response).
            let expires_at = time::OffsetDateTime::from_unix_timestamp(user_session.expires_at)
                .map_err(|e| RestError::InternalError(format!("invalid expires_at: {}", e)))?
                .format(&time::format_description::well_known::Iso8601::DEFAULT)
                .map_err(|e| RestError::InternalError(format!("format expires_at: {}", e)))?;

            let body = HelperSessionTO {
                assembly_id: info.assembly_id,
                assembly_name: info.assembly_name.to_string(),
                expires_at,
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&body)?))
                .unwrap())
        })
        .await,
    )
}

/// POST /api/helper/logout — invalidates the current Helper-Session (D-07).
/// Sets `app_session=; Max-Age=0` to clear the cookie client-side AND
/// invalidates the session server-side via `SessionService::invalidate_session`.
///
/// T-04-03 Mitigation: only the cookie's session_id is honoured (no body /
/// query parameter parsing). Browsers can only emit their own cookie.
#[instrument(skip(rest_state, headers))]
#[utoipa::path(
    post,
    tag = "Helper Session",
    path = "/logout",
    responses(
        (status = 204, description = "Session invalidated"),
        (status = 401, description = "No helper session present"),
    ),
)]
pub async fn helper_logout<RestState: RestStateDef + HelperTokenRestState>(
    rest_state: State<RestState>,
    headers: HeaderMap,
) -> Response {
    error_handler(
        (async {
            let session_id_str = read_session_cookie(&headers).ok_or(RestError::Unauthorized)?;

            // Verify the session is real and is a Helper-Session before doing
            // anything destructive — otherwise an attacker could probe for
            // valid session-ids by observing the response code. We require
            // BOTH the SessionService check AND the helper_token reverse
            // lookup so admin/OIDC sessions are rejected with 401 (T-04-02).
            rest_state
                .session_service()
                .verify_user_session(&session_id_str)
                .await
                .map_err(|_| RestError::Unauthorized)?
                .ok_or(RestError::Unauthorized)?;
            rest_state
                .helper_token_service()
                .find_assembly_for_session(&session_id_str)
                .await
                .map_err(|_| RestError::Unauthorized)?
                .ok_or(RestError::Unauthorized)?;

            // Server-side invalidation. Errors are logged but not surfaced —
            // the cookie-override below is authoritative for the client. A
            // failure here at most leaves an orphan session row that the
            // session cleanup worker eventually reaps.
            if let Err(e) = rest_state
                .session_service()
                .invalidate_session(&session_id_str)
                .await
            {
                tracing::warn!(error = ?e, "session invalidation failed during logout");
            }

            // Cookie-Override Max-Age=0 (mirror of the redeem Set-Cookie
            // attributes from helper_token.rs:317-319).
            let cookie_value = "app_session=; Path=/; HttpOnly; SameSite=Strict; Secure; Max-Age=0";

            Ok(Response::builder()
                .status(204)
                .header(
                    header::SET_COOKIE,
                    HeaderValue::from_str(cookie_value).map_err(|e| {
                        RestError::InternalError(format!("invalid cookie value: {}", e))
                    })?,
                )
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

// ============================================================================
// Router-Funktionen
// ============================================================================

pub fn generate_route<RestState: RestStateDef + HelperTokenRestState>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            get(list_helper_tokens::<RestState>).post(create_helper_token::<RestState>),
        )
        .route("/{token_id}/revoke", post(revoke_helper_token::<RestState>))
}

pub fn generate_public_route<RestState: RestStateDef + HelperTokenRestState>() -> Router<RestState>
{
    // Append-only: lib.rs:711 mounts THIS router on /api/helper. A second
    // `.nest("/api/helper", ...)` would shadow this one. Phase 4 Plan 01
    // adds /session (D-06) + /logout (D-07) here.
    Router::new()
        .route("/redeem", post(redeem_helper_token::<RestState>))
        .route("/session", get(get_helper_session::<RestState>))
        .route("/logout", post(helper_logout::<RestState>))
}

// ============================================================================
// OpenAPI ApiDocs
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    paths(create_helper_token, list_helper_tokens, revoke_helper_token),
    components(schemas(
        HelperTokenTO,
        HelperTokenStatusTO,
        HelperTokenCreateResponseTO,
        CreateHelperTokenRequest
    ))
)]
pub struct ApiDoc;

#[derive(OpenApi)]
#[openapi(
    paths(redeem_helper_token, get_helper_session, helper_logout),
    components(schemas(RedeemRequest, RedeemResponse, HelperSessionTO))
)]
pub struct PublicApiDoc;

// ============================================================================
// Tests (Validation only — handler-level e2e in Plan 08)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Security-Regression-Guard (instrument-skip-redeem-code): der Klartext-
    /// Redeem-Code (`RedeemRequest.code`, `#[derive(Debug)]`) darf NICHT im
    /// Tracing-Span landen. Das `skip(...)` der `#[instrument]`-Annotation IST
    /// das Verhalten (compile-time), deshalb wird es hier auf Quellebene
    /// abgesichert: entfernt jemand `body` aus dem skip, schlägt der Test fehl.
    /// Ein Laufzeit-Test ist nicht praktikabel, weil Server-Handler-Spans in
    /// eigenen Worker-Threads laufen (thread-lokaler Capture greift dort nicht).
    #[test]
    fn test_redeem_handler_skips_body_in_tracing_span() {
        let src = include_str!("helper_token.rs");
        let fn_idx = src
            .find("pub async fn redeem_helper_token")
            .expect("redeem_helper_token handler not found");
        // Die dem Handler unmittelbar vorausgehende #[instrument(...)]-Zeile.
        let attr_start = src[..fn_idx]
            .rfind("#[instrument(")
            .expect("#[instrument] before redeem_helper_token not found");
        let line_len = src[attr_start..]
            .find('\n')
            .expect("instrument attribute must be single-line");
        let instrument_line = &src[attr_start..attr_start + line_len];
        assert!(
            instrument_line.contains("body"),
            "redeem_helper_token muss `body` im instrument(skip(...)) ausschließen, \
             sonst landet der Klartext-Redeem-Code im Tracing-Span. Gefunden: {instrument_line}"
        );
    }

    #[test]
    fn test_validate_create_helper_token_request_valid() {
        let body = CreateHelperTokenRequest {
            memo: "Anna".to_string(),
        };
        assert!(validate_create_helper_token_request(&body).is_ok());
    }

    #[test]
    fn test_validate_create_helper_token_request_empty_memo() {
        let body = CreateHelperTokenRequest {
            memo: "".to_string(),
        };
        let err = validate_create_helper_token_request(&body).unwrap_err();
        assert!(err
            .iter()
            .any(|e| e.field.as_ref() == "memo" && e.message.as_ref() == "missing"));
    }

    #[test]
    fn test_validate_create_helper_token_request_whitespace_memo() {
        let body = CreateHelperTokenRequest {
            memo: "   ".to_string(),
        };
        let err = validate_create_helper_token_request(&body).unwrap_err();
        assert!(err.iter().any(|e| e.field.as_ref() == "memo"));
    }

    #[test]
    fn test_validate_create_helper_token_request_too_long_memo() {
        let body = CreateHelperTokenRequest {
            memo: "a".repeat(257),
        };
        let err = validate_create_helper_token_request(&body).unwrap_err();
        assert!(err
            .iter()
            .any(|e| e.field.as_ref() == "memo" && e.message.as_ref().contains("too_long")));
    }
}
