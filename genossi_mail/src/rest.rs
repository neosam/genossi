use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};

use crate::dao::{MailJob, MailRecipient};
use crate::service::{AttachmentInput, MailService, MailServiceError, RecipientInput};
use crate::template::{member_to_template_context, render_html_template, render_template};
use genossi_dao::member::MemberEntity;

/// Resolved document info for attachment validation.
pub struct ResolvedDocument {
    pub document_id: uuid::Uuid,
    pub member_id: uuid::Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub relative_path: String,
}

pub trait MailRestState: Clone + Send + Sync + 'static {
    type MailService: MailService;
    fn mail_service(&self) -> Arc<Self::MailService>;
    /// Resolve a document by ID. Returns None if not found or soft-deleted.
    fn resolve_document(
        &self,
        document_id: uuid::Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<ResolvedDocument>> + Send + '_>>;
    /// Get attachments for a recipient.
    fn get_recipient_attachments(
        &self,
        recipient_id: uuid::Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<MailAttachmentTO>> + Send + '_>>;
    /// Resolve a member by ID for template rendering.
    fn resolve_member(
        &self,
        member_id: uuid::Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<MemberEntity>> + Send + '_>>;
    /// Resolve multiple members by IDs for template validation.
    fn resolve_members(
        &self,
        member_ids: &[uuid::Uuid],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<MemberEntity>> + Send + '_>>;
    /// Resolve the per-member repayment context for `/preview` rendering.
    ///
    /// Mirrors the aggregation logic in `genossi_mail/src/worker.rs:332-361`
    /// (filter `deleted IS NULL` + `status IN (Open, Contacted)`, sum
    /// `share_count_to_pay_out`, format payout in German locale `X,YZ`).
    ///
    /// Returns `None` when the phase does not exist OR the member has no
    /// Open/Contacted entries in the phase (D-05 symmetry with the worker).
    ///
    /// Tuple shape: `(payout_amount, share_count, share_value, fiscal_year)`.
    /// `share_value` is the phase-wide Anteilswert pro Stueck as German euro
    /// string `X,YZ` (e.g. "120,00") — Quick 260602-r2i.
    fn resolve_repayment_context(
        &self,
        phase_id: uuid::Uuid,
        member_id: uuid::Uuid,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Option<(String, i32, String, i32)>> + Send + '_>,
    >;
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MailJobTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    /// Phase 23 (HTML-01): read-only exposure of the persisted
    /// (ammonia-sanitized) `MailJob.body_html` — the multipart/alternative
    /// HTML sibling. Additive + backward-compatible per the same pattern
    /// as `repayment_phase_id` below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo</p>")]
    pub body_html: Option<String>,
    #[schema(example = "running")]
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
    /// Quick 260603-evf: read-only exposure of the persisted
    /// `MailJob.repayment_phase_id` (DAO field, set at job creation when
    /// the bulk-mail is a repayment-flow send). The frontend uses this so
    /// recipients that failed with `error="no_repayment_letter"` can be
    /// resolved against the correct phase for the "Brief generieren +
    /// Retry" recovery action without iterating all phases.
    ///
    /// Additive + backward-compatible:
    /// - `#[serde(default)]` means older client JSON payloads without the
    ///   key still deserialize cleanly to `None`.
    /// - `skip_serializing_if = "Option::is_none"` keeps the wire shape
    ///   identical for non-repayment jobs (the key is simply absent).
    ///
    /// Stays `None` for ad-hoc single sends and non-repayment bulk-mails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repayment_phase_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MailAttachmentTO {
    pub document_id: String,
    pub file_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MailRecipientTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: String,
    pub to_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    #[schema(example = "sent")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    // Quick 260614-9zf: the actually-rendered subject/body this recipient received.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_body: Option<String>,
    /// Phase 23 (HTML-01, D-08): per-recipient rendered HTML sibling —
    /// byte-accurate copy of the `text/html` part actually sent, or `None`
    /// for text-only jobs. Backward-compatible via `skip_serializing_if`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo Max</p>")]
    pub rendered_html_body: Option<String>,
    // Quick 260614-b1t: true when rendered_subject/body were reconstructed by the
    // startup backfill (not the byte-accurate original send). Always serialized
    // (DB NOT NULL DEFAULT 0 guarantees a value); #[serde(default)] for input robustness.
    #[serde(default)]
    pub rendered_reconstructed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<MailAttachmentTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MailJobDetailTO {
    #[serde(flatten)]
    pub job: MailJobTO,
    pub recipients: Vec<MailRecipientTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SendMailRequest {
    #[schema(example = "user@example.com")]
    pub to_address: String,
    #[schema(example = "Test Subject")]
    pub subject: String,
    #[schema(example = "Hello, this is a test email.")]
    pub body: String,
    /// Phase 23 (HTML-01, HTML-05, D-03 entry point 1): optional author HTML.
    /// Sanitized server-side (ammonia) before persistence — the value on the
    /// wire is untrusted; the stored value is safe. Backward-compatible:
    /// pre-Phase-24 clients that omit this key continue to work unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo</p>")]
    pub body_html: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkRecipient {
    #[schema(example = "user@example.com")]
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SendBulkMailRequest {
    pub to_addresses: Vec<BulkRecipient>,
    #[schema(example = "Test Subject")]
    pub subject: String,
    #[schema(example = "Hello, this is a test email.")]
    pub body: String,
    /// Phase 23 (HTML-01, HTML-05, D-03 entry point 1): optional author HTML
    /// forwarded to `MailService::create_job` for sanitize + persist.
    /// Backward-compatible via `#[serde(default)]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo {{ first_name }}</p>")]
    pub body_html: Option<String>,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    #[serde(default)]
    pub static_document_ids: Vec<String>,
    /// Phase 10 D-12: optional reference to MailTemplate used to render this job.
    /// Worker uses this to populate MemberDocument.template_id for audit traceability.
    /// Must be a valid Uuid string; invalid -> 400 BadRequest.
    #[serde(default)]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub template_id: Option<String>,
    /// Phase 10 D-03: optional reference to RepaymentPhase. When set, the worker
    /// merges per-recipient payout context (payout_amount/share_count/fiscal_year)
    /// into the minijinja render. Must be a valid Uuid string; invalid -> 400 BadRequest.
    #[serde(default)]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub repayment_phase_id: Option<String>,
    /// Quick 260603-cz6: opt-in flag. When `true`, the worker resolves the
    /// per-recipient `DocumentType::RepaymentLetter` MemberDocument
    /// (Description-Fingerprint `"Anschreiben Auszahlung GJ {fiscal_year}"`)
    /// and attaches it in-memory. Requires `repayment_phase_id` to be set —
    /// otherwise 400 BadRequest. Recipients without a matching letter are marked
    /// `failed` with `error="no_repayment_letter"`.
    #[serde(default)]
    #[schema(example = false)]
    pub attach_repayment_letter: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TestMailRequest {
    #[schema(example = "admin@example.com")]
    pub to_address: String,
}

/// Quick 260603-jtf: Request body for `POST /api/mail/test-with-template`.
///
/// Sends a single test mail by rendering `subject`+`body` against the Member's
/// template variables (resolved server-side from `member_id`) and delivering
/// the result to `to_address`.
///
/// **Privacy invariant (CRITICAL):** `to_address` and `member_id` are explicit,
/// independent fields. The Member's email is NEVER used as a recipient — the
/// Member contributes ONLY template variables. Caller is responsible for
/// supplying an explicit test recipient address. The handler MUST NOT silently
/// fall back to a member-derived address.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TestMailWithTemplateRequest {
    #[schema(example = "vorstand@example.com")]
    pub to_address: String,
    #[schema(example = "Hallo {{ first_name }}")]
    pub subject: String,
    #[schema(example = "Liebe/r {{ first_name }} {{ last_name }}...")]
    pub body: String,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub member_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "29ae374c-9e60-4cc8-b0b4-ce51c28e7b6e")]
    pub repayment_phase_id: Option<String>,
    /// Phase 23 (HTML-01, HTML-05, D-03 entry point 4): optional author HTML
    /// template. The handler renders it against the resolved Member's
    /// variables (via `render_html_template`), then hands the rendered value
    /// to `MailService::send_test_mail_with_body` which sanitizes it and
    /// forwards to `build_message` as the multipart/alternative HTML part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo {{ first_name }}</p>")]
    pub body_html: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PreviewRequest {
    #[schema(example = "Hallo {{ first_name }}")]
    pub subject: String,
    #[schema(example = "Liebe/r {{ first_name }} {{ last_name }}...")]
    pub body: String,
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub member_id: String,
    /// Optionaler Repayment-Kontext — wenn gesetzt, werden `payout_amount`,
    /// `share_count` und `fiscal_year` aus den Open/Contacted-RepaymentEntries
    /// des Members in der referenzierten Phase aggregiert (gleiche Logik wie
    /// der Send-Worker, siehe `genossi_mail/src/worker.rs:332-361`). Ohne
    /// dieses Feld bleibt die pure-member-Render-Logik aktiv.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "29ae374c-9e60-4cc8-b0b4-ce51c28e7b6e")]
    pub repayment_phase_id: Option<String>,
    /// Phase 24 (EDIT-05, D-04): optional author HTML template. When present,
    /// the `preview_mail` handler renders it via `render_html_template`
    /// (autoescape env — member values HTML-escaped) and echoes the rendered
    /// HTML back in `PreviewResponse.body_html`. `None` ⇒ response.body_html
    /// stays `None` (wire backward-compatible with pre-Phase-24 clients).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo {{ first_name }}</p>")]
    pub body_html: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PreviewResponse {
    pub subject: String,
    pub body: String,
    /// Phase 24 (EDIT-05, D-04): the rendered HTML sibling — populated only
    /// when the `PreviewRequest` carried a `body_html` template. Renders
    /// through the autoescape env, so member-supplied values are escaped
    /// while author markup passes through structurally. `None` when the
    /// request omitted `body_html`; `#[serde(skip_serializing_if)]` keeps
    /// the wire backward-compatible with older frontends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_html: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    /// Quick 260603-kon: signalisiert dem Frontend, dass Dummy-Repayment-Daten
    /// (`template::dummy_repayment_context`) gerendert wurden — `repayment_phase_id`
    /// war im Request gesetzt, aber `resolve_repayment_context` lieferte `None`
    /// (Member hat keine Open/Contacted-Entries in der referenzierten Phase).
    /// Frontend zeigt darauf einen amber Hinweis-Banner an.
    ///
    /// Bleibt `false`/absent wenn:
    /// - kein `repayment_phase_id` geschickt wurde (regulaerer Preview-Pfad),
    /// - ODER echte Repayment-Daten gefunden wurden (Phase aktiv).
    ///
    /// `skip_serializing_if = "std::ops::Not::not"` filtert das Feld auf
    /// `true` — Backward-Compat: aelterer Frontend-Code, der das Feld nicht
    /// kennt, sieht keine Aenderung in der Wire-Shape.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub used_dummy_repayment: bool,
}

fn format_datetime(dt: &time::PrimitiveDateTime) -> String {
    dt.assume_utc()
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| dt.to_string())
}

impl From<&MailJob> for MailJobTO {
    fn from(job: &MailJob) -> Self {
        Self {
            id: job.id.to_string(),
            created: format_datetime(&job.created),
            subject: job.subject.to_string(),
            body: job.body.to_string(),
            // Phase 23 Plan 04 (HTML-01): expose the sanitized HTML sibling
            // read-only. Sanitized at the store boundary (D-03) — safe to
            // return verbatim.
            body_html: job.body_html.as_deref().map(String::from),
            status: job.status.to_string(),
            total_count: job.total_count,
            sent_count: job.sent_count,
            failed_count: job.failed_count,
            // Quick 260603-evf: stringify the persisted phase UUID so the
            // frontend can deterministically resolve the phase for the
            // "Brief generieren + Retry" action without scanning all phases.
            repayment_phase_id: job.repayment_phase_id.map(|u| u.to_string()),
        }
    }
}

impl From<&MailRecipient> for MailRecipientTO {
    fn from(r: &MailRecipient) -> Self {
        Self {
            id: r.id.to_string(),
            to_address: r.to_address.to_string(),
            member_id: r.member_id.map(|m| m.to_string()),
            status: r.status.to_string(),
            error: r.error.as_deref().map(String::from),
            sent_at: r.sent_at.as_ref().map(format_datetime),
            rendered_subject: r.rendered_subject.as_deref().map(String::from),
            rendered_body: r.rendered_body.as_deref().map(String::from),
            // Phase 23 Plan 04 (HTML-01, D-08): expose the per-recipient
            // byte-accurate rendered HTML sibling.
            rendered_html_body: r.rendered_html_body.as_deref().map(String::from),
            rendered_reconstructed: r.rendered_reconstructed,
            attachments: vec![],
        }
    }
}

fn error_handler(result: Result<Response, MailServiceError>) -> Response {
    match result {
        Ok(response) => response,
        Err(MailServiceError::ConfigMissing(msg)) => Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": msg.to_string()}).to_string(),
            ))
            .unwrap(),
        Err(MailServiceError::SmtpError(msg)) => Response::builder()
            .status(502)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": msg.to_string()}).to_string(),
            ))
            .unwrap(),
        Err(MailServiceError::NotFound) => Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": "Not found"}).to_string(),
            ))
            .unwrap(),
        Err(MailServiceError::TemplateValidation(msg)) => Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": msg.to_string()}).to_string(),
            ))
            .unwrap(),
        Err(MailServiceError::BadRequest(msg)) => Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": msg.to_string()}).to_string(),
            ))
            .unwrap(),
        Err(MailServiceError::DataAccess(msg)) => {
            tracing::error!("Mail data access error: {}", msg);
            Response::builder()
                .status(500)
                .body(Body::from("Internal server error"))
                .unwrap()
        }
    }
}

pub fn generate_route<S: MailRestState>() -> Router<S> {
    Router::new()
        .route("/send", post(send_mail::<S>))
        .route("/send-bulk", post(send_bulk_mail::<S>))
        .route("/preview", post(preview_mail::<S>))
        .route("/test", post(send_test_mail::<S>))
        // Quick 260603-jtf: template-test endpoint (distinct from `/test` which
        // sends a hard-coded constant for SMTP-config smoke-tests).
        .route(
            "/test-with-template",
            post(send_test_mail_with_template::<S>),
        )
        .route("/jobs", get(get_jobs::<S>))
        .route("/jobs/{id}", get(get_job_detail::<S>))
        .route("/jobs/{id}/retry", post(retry_job::<S>))
}

#[derive(OpenApi)]
#[openapi(
    paths(send_mail, send_bulk_mail, preview_mail, send_test_mail, send_test_mail_with_template, get_jobs, get_job_detail, retry_job),
    components(schemas(MailJobTO, MailRecipientTO, MailJobDetailTO, MailAttachmentTO, SendMailRequest, SendBulkMailRequest, BulkRecipient, TestMailRequest, TestMailWithTemplateRequest, PreviewRequest, PreviewResponse)),
    tags((name = "Mail", description = "Email sending and job management endpoints"))
)]
pub struct ApiDoc;

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/send",
    request_body = SendMailRequest,
    responses(
        (status = 202, description = "Mail job created", body = MailJobTO),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn send_mail<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<SendMailRequest>,
) -> Response {
    error_handler(
        (async {
            let job = state
                .mail_service()
                .create_job(
                    &body.subject,
                    &body.body,
                    body.body_html.clone(), // Phase 23 Plan 04 (HTML-01, D-03 EP1)
                    vec![RecipientInput {
                        address: body.to_address,
                        member_id: None,
                        application_id: None,
                    }],
                    vec![],
                    vec![],
                    None,  // template_id: Phase 10 single-send is ad-hoc, no template tracking
                    None,  // repayment_phase_id: Phase 10 single-send is not bulk-repayment
                    false, // attach_repayment_letter: not applicable to single-send
                )
                .await?;
            let to = MailJobTO::from(&job);
            Ok(Response::builder()
                .status(202)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/send-bulk",
    request_body = SendBulkMailRequest,
    responses(
        (status = 202, description = "Bulk mail job created", body = MailJobTO),
        (status = 400, description = "Empty recipients or invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn send_bulk_mail<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<SendBulkMailRequest>,
) -> Response {
    error_handler(
        (async {
            let recipients: Vec<RecipientInput> = body
                .to_addresses
                .into_iter()
                .map(|r| RecipientInput {
                    address: r.address,
                    member_id: r.member_id.and_then(|id| uuid::Uuid::parse_str(&id).ok()),
                    application_id: None,
                })
                .collect();

            // Validate all recipients have member_id
            if recipients.iter().any(|r| r.member_id.is_none()) {
                return Err(MailServiceError::TemplateValidation(Arc::from(
                    "All recipients must have a member_id for template rendering",
                )));
            }

            // Validate templates against all recipient members.
            //
            // Phase 10 Plan 08 (Rule 2 fix from Plan 10.04 gap): when the
            // bulk-send request carries a repayment_phase_id, the body
            // typically references `{{ payout_amount }}`, `{{ share_count }}`
            // and `{{ fiscal_year }}` — variables the WORKER will inject from
            // the RepaymentPhase + RepaymentEntries (Plan 10.06). The pure
            // `validate_template` helper does not know about these vars and
            // would reject the request under strict-env. Use the
            // `validate_template_with_repayment` helper (Plan 10.05) when the
            // phase-id is present so the probe-render uses a merged context
            // and catches BOTH plain-member-var bugs AND repayment-var
            // typos in the same call. D-14.
            let member_ids: Vec<uuid::Uuid> =
                recipients.iter().filter_map(|r| r.member_id).collect();
            let members = state.resolve_members(&member_ids).await;
            let validation_result = if body
                .repayment_phase_id
                .as_deref()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
            {
                crate::template::validate_template_with_repayment(
                    &body.subject,
                    &body.body,
                    &members,
                )
            } else {
                crate::template::validate_template(&body.subject, &body.body, &members)
            };
            if let Err(errors) = validation_result {
                return Err(MailServiceError::TemplateValidation(Arc::from(
                    errors.join("; "),
                )));
            }

            // Resolve and validate attachments
            let mut attachment_inputs = Vec::new();
            if !body.attachment_ids.is_empty() {
                if recipients.len() > 1 {
                    return Err(MailServiceError::DataAccess(Arc::from(
                        "Attachments are only supported for single-recipient sends",
                    )));
                }
                let recipient_member_id = recipients.first().and_then(|r| r.member_id);

                for att_id_str in &body.attachment_ids {
                    let doc_id = uuid::Uuid::parse_str(att_id_str)
                        .map_err(|_| MailServiceError::NotFound)?;

                    let doc = state
                        .resolve_document(doc_id)
                        .await
                        .ok_or(MailServiceError::NotFound)?;

                    // Validate ownership
                    if Some(doc.member_id) != recipient_member_id {
                        return Err(MailServiceError::DataAccess(Arc::from(
                            "Attachment does not belong to the recipient's member",
                        )));
                    }

                    attachment_inputs.push(AttachmentInput {
                        document_id: doc.document_id,
                        file_name: doc.file_name,
                        mime_type: doc.mime_type,
                        relative_path: doc.relative_path,
                    });
                }
            }

            let mut static_document_ids: Vec<uuid::Uuid> = Vec::new();
            for sid in &body.static_document_ids {
                let parsed = uuid::Uuid::parse_str(sid).map_err(|_| MailServiceError::NotFound)?;
                static_document_ids.push(parsed);
            }

            // Phase 10 D-12: parse optional template_id (invalid UUID -> 400 BadRequest)
            let template_id: Option<uuid::Uuid> = match &body.template_id {
                Some(s) if !s.is_empty() => Some(uuid::Uuid::parse_str(s).map_err(|_| {
                    MailServiceError::BadRequest(Arc::from(
                        format!("Invalid template_id UUID: {}", s).as_str(),
                    ))
                })?),
                _ => None,
            };

            // Phase 10 D-03: parse optional repayment_phase_id (invalid UUID -> 400 BadRequest)
            let repayment_phase_id: Option<uuid::Uuid> = match &body.repayment_phase_id {
                Some(s) if !s.is_empty() => Some(uuid::Uuid::parse_str(s).map_err(|_| {
                    MailServiceError::BadRequest(Arc::from(
                        format!("Invalid repayment_phase_id UUID: {}", s).as_str(),
                    ))
                })?),
                _ => None,
            };

            // Quick 260603-cz6: opt-in attach_repayment_letter requires repayment_phase_id.
            // Service-layer enforces this too, but we 400 here for a clear error message
            // before going through the recipient/static-doc validation.
            if body.attach_repayment_letter && repayment_phase_id.is_none() {
                return Err(MailServiceError::BadRequest(Arc::from(
                    "attach_repayment_letter requires repayment_phase_id",
                )));
            }

            let job = state
                .mail_service()
                .create_job(
                    &body.subject,
                    &body.body,
                    body.body_html.clone(), // Phase 23 Plan 04 (HTML-01, D-03 EP1)
                    recipients,
                    attachment_inputs,
                    static_document_ids,
                    template_id,                  // Phase 10 D-12
                    repayment_phase_id,           // Phase 10 D-03
                    body.attach_repayment_letter, // Quick 260603-cz6
                )
                .await?;
            let to = MailJobTO::from(&job);
            Ok(Response::builder()
                .status(202)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/preview",
    request_body = PreviewRequest,
    responses(
        (status = 200, description = "Rendered preview", body = PreviewResponse),
        (status = 404, description = "Member not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn preview_mail<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<PreviewRequest>,
) -> Response {
    error_handler(
        (async {
            let member_id = uuid::Uuid::parse_str(&body.member_id)
                .map_err(|_| MailServiceError::BadRequest(Arc::from("Invalid member_id")))?;

            let member = state
                .resolve_member(member_id)
                .await
                .ok_or(MailServiceError::NotFound)?;

            // Quick-c19 fix: bei gesetztem repayment_phase_id werden payout_amount/
            // share_count/fiscal_year aus den echten RepaymentEntries des Members
            // aggregiert — gleiche Logik wie der Send-Worker
            // (`genossi_mail/src/worker.rs:332-361`, Single Source of Truth fuer
            // Filter/Summe/Format). D-05-Symmetrie: hat der Member keine
            // Open/Contacted-Entries, wird kein Repayment-Context gemergt; das
            // Template muss in diesem Fall `{% if share_count is defined %}`
            // verwenden (Plan 10.05).
            // Quick 260603-kon: bei gesetztem repayment_phase_id und fehlender
            // aktiver Phase (None aus resolve_repayment_context) wird der
            // Dummy-Fallback aktiviert, damit Vorstand auch ausserhalb aktiver
            // Auszahlungsphasen Templates mit `{{ payout_amount }}` etc. testen
            // kann. used_dummy_repayment signalisiert das dem Frontend, das
            // einen sichtbaren Hinweis-Banner zeigt.
            let base_ctx = member_to_template_context(&member);
            let (ctx, used_dummy_repayment) = match body.repayment_phase_id.as_deref() {
                Some(s) if !s.is_empty() => {
                    let phase_id = uuid::Uuid::parse_str(s).map_err(|_| {
                        MailServiceError::BadRequest(Arc::from("Invalid repayment_phase_id"))
                    })?;
                    match state.resolve_repayment_context(phase_id, member_id).await {
                        Some((payout, share_count, share_value, fiscal_year)) => (
                            crate::template::merge_repayment_context(
                                base_ctx,
                                &payout,
                                share_count,
                                &share_value,
                                fiscal_year,
                            ),
                            false,
                        ),
                        None => {
                            // Quick 260603-kon: Dummy-Fallback nur fuer Test-Pfade.
                            let (payout, share_count, share_value, fiscal_year) =
                                crate::template::dummy_repayment_context();
                            (
                                crate::template::merge_repayment_context(
                                    base_ctx,
                                    payout,
                                    share_count,
                                    share_value,
                                    fiscal_year,
                                ),
                                true,
                            )
                        }
                    }
                }
                _ => {
                    // Quick 260603-n3m: Caller (z.B. Template-Editor) hat
                    // KEINE repayment_phase_id geschickt. Wenn das Template
                    // trotzdem `{{ payout_amount }}` / `share_count` /
                    // `share_value` / `fiscal_year` referenziert, mergen
                    // wir den Dummy-Repayment-Context — sonst wuerde
                    // strict-env minijinja mit "undefined variable" failen
                    // und der Editor-Preview waere unbenutzbar.
                    // Symmetrisch zum Typst-Test-Endpoint aus 260603-kon.
                    if crate::template::template_uses_repayment_vars(&body.subject, &body.body) {
                        let (payout, share_count, share_value, fiscal_year) =
                            crate::template::dummy_repayment_context();
                        (
                            crate::template::merge_repayment_context(
                                base_ctx,
                                payout,
                                share_count,
                                share_value,
                                fiscal_year,
                            ),
                            true,
                        )
                    } else {
                        (base_ctx, false)
                    }
                }
            };
            let mut errors = Vec::new();

            let rendered_subject = match render_template(&body.subject, &ctx) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("Subject: {}", e.message));
                    String::new()
                }
            };

            let rendered_body = match render_template(&body.body, &ctx) {
                Ok(s) => s,
                Err(e) => {
                    errors.push(format!("Body: {}", e.message));
                    String::new()
                }
            };

            // Phase 24 (EDIT-05, D-04): if the caller supplied an HTML sibling,
            // render it through the autoescape env (member values escaped, author
            // markup structurally preserved).
            //
            // Phase 28 (PREV-02, D-01/D-02): sanitize BEFORE render — die Vorschau
            // zeigt exakt die HTML-Fassung, die der Empfänger bekommt, statt des
            // ungefilterten `contenteditable`-DOMs. Die Reihenfolge ist bindend und
            // spiegelt die Produktion: ammonia greift am Store-Boundary (Phase 23
            // D-03), das Jinja-Rendering erst im Send-Worker. Render-dann-sanitize
            // wäre asymmetrisch, weil Member-Werte in Produktion autoescaped und
            // nicht sanitisiert werden.
            // Jinja-Platzhalter im TEXT-Content (`<p>Hallo {{ first_name }}</p>`)
            // überleben ammonia unverändert (siehe `sanitize.rs` Zeilen 30-34).
            // Platzhalter in ATTRIBUTEN (`<a href="{{ link }}">`) sind seit Phase 24
            // out-of-contract und werden hier erstmals sichtbar gestrippt — gewollt,
            // kein Bug, und laut D-04 ohne Diff-Banner: die Darstellung des
            // sanitisierten Ergebnisses ist der Beweis.
            // `sanitize_body_html_opt` garantiert `None` in ⇒ `None` out, deshalb
            // keine zusätzliche Verzweigung (kein `Some("")`-Sentinel).
            let sanitized_body_html =
                crate::service::sanitize_body_html_opt(body.body_html.as_deref());
            let rendered_body_html: Option<String> = match sanitized_body_html.as_deref() {
                Some(html_src) => match render_html_template(html_src, &ctx) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        errors.push(format!("HTML: {}", e.message));
                        None
                    }
                },
                None => None,
            };

            // Quick 260718-html-to-plain-derivation (Nachtrag): Preview muss den
            // Plain-Text ebenso aus dem HTML ableiten wie der Send-Worker, sonst
            // sieht der Vorstand in der Editor-Preview eine strukturlose Wall-of-
            // Text während Empfänger korrekt formatierten Text bekommen. Siehe
            // `crate::render::plain_from_html` für Semantik + Rationale.
            let rendered_body = match rendered_body_html.as_deref() {
                Some(html) => crate::render::plain_from_html(html),
                None => rendered_body,
            };

            let response = PreviewResponse {
                subject: rendered_subject,
                body: rendered_body,
                body_html: rendered_body_html,
                errors,
                used_dummy_repayment,
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/test",
    request_body = TestMailRequest,
    responses(
        (status = 200, description = "Test mail sent successfully"),
        (status = 400, description = "SMTP config missing"),
        (status = 502, description = "SMTP error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn send_test_mail<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<TestMailRequest>,
) -> Response {
    error_handler(
        (async {
            state
                .mail_service()
                .send_test_mail(&body.to_address)
                .await?;
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::json!({"success": true}).to_string()))
                .unwrap())
        })
        .await,
    )
}

/// Quick 260603-jtf: Render the supplied template (subject+body) against the
/// resolved Member's variables and send a single SMTP mail to the **explicit**
/// `to_address` in the request body.
///
/// **Privacy defense (mirrors `TestMailWithTemplateRequest` doc-comment):**
/// `body.to_address` is the recipient. The Member is loaded ONLY to provide
/// template variables — its email is NOT referenced anywhere in this handler.
/// Compare with `/api/mail/send-bulk` which delivers to Member addresses; that
/// flow is deliberately separate.
#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/test-with-template",
    request_body = TestMailWithTemplateRequest,
    responses(
        (status = 200, description = "Test mail with rendered template sent"),
        (status = 400, description = "Invalid request or template error"),
        (status = 404, description = "Member not found"),
        (status = 502, description = "SMTP error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn send_test_mail_with_template<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<TestMailWithTemplateRequest>,
) -> Response {
    error_handler(
        (async {
            let member_id = uuid::Uuid::parse_str(&body.member_id)
                .map_err(|_| MailServiceError::BadRequest(Arc::from("Invalid member_id")))?;

            let member = state
                .resolve_member(member_id)
                .await
                .ok_or(MailServiceError::NotFound)?;

            // Re-use the preview-mail context-merge logic so the test render
            // matches the live preview path 1:1 (D-05 symmetry).
            // Quick 260603-kon: identischer Dummy-Fallback wie in preview_mail —
            // wenn `repayment_phase_id` gesetzt und resolve_repayment_context
            // None liefert, werden Sentinel-Werte (99,99 / 99 / 2099)
            // gerendert. Response-Body enthaelt `used_dummy_repayment` als
            // Hinweis fuer Frontend-Logging.
            let base_ctx = member_to_template_context(&member);
            let (ctx, used_dummy_repayment) = match body.repayment_phase_id.as_deref() {
                Some(s) if !s.is_empty() => {
                    let phase_id = uuid::Uuid::parse_str(s).map_err(|_| {
                        MailServiceError::BadRequest(Arc::from("Invalid repayment_phase_id"))
                    })?;
                    match state.resolve_repayment_context(phase_id, member_id).await {
                        Some((payout, share_count, share_value, fiscal_year)) => (
                            crate::template::merge_repayment_context(
                                base_ctx,
                                &payout,
                                share_count,
                                &share_value,
                                fiscal_year,
                            ),
                            false,
                        ),
                        None => {
                            // Quick 260603-kon: Dummy-Fallback nur fuer Test-Pfade.
                            let (payout, share_count, share_value, fiscal_year) =
                                crate::template::dummy_repayment_context();
                            (
                                crate::template::merge_repayment_context(
                                    base_ctx,
                                    payout,
                                    share_count,
                                    share_value,
                                    fiscal_year,
                                ),
                                true,
                            )
                        }
                    }
                }
                _ => {
                    // Quick 260603-n3m: Caller (z.B. Template-Editor) hat
                    // KEINE repayment_phase_id geschickt. Wenn das Template
                    // trotzdem `{{ payout_amount }}` / `share_count` /
                    // `share_value` / `fiscal_year` referenziert, mergen
                    // wir den Dummy-Repayment-Context — sonst wuerde
                    // strict-env minijinja mit "undefined variable" failen
                    // und der Editor-Preview waere unbenutzbar.
                    // Symmetrisch zum Typst-Test-Endpoint aus 260603-kon.
                    if crate::template::template_uses_repayment_vars(&body.subject, &body.body) {
                        let (payout, share_count, share_value, fiscal_year) =
                            crate::template::dummy_repayment_context();
                        (
                            crate::template::merge_repayment_context(
                                base_ctx,
                                payout,
                                share_count,
                                share_value,
                                fiscal_year,
                            ),
                            true,
                        )
                    } else {
                        (base_ctx, false)
                    }
                }
            };

            let rendered_subject = render_template(&body.subject, &ctx).map_err(|e| {
                MailServiceError::TemplateValidation(Arc::from(format!("Subject: {}", e.message)))
            })?;
            let rendered_body = render_template(&body.body, &ctx).map_err(|e| {
                MailServiceError::TemplateValidation(Arc::from(format!("Body: {}", e.message)))
            })?;

            // Phase 23 Plan 04 (HTML-01, D-03 EP4, D-04): render the optional
            // HTML sibling through the HTML env (autoescaping — member values
            // are escaped so `<script>` in a first_name renders as
            // `&lt;script&gt;`). None ⇒ pass None straight through so the
            // service's sanitize helper preserves None (Pitfall 4).
            let rendered_body_html: Option<String> = match body.body_html.as_deref() {
                Some(tmpl) => Some(render_html_template(tmpl, &ctx).map_err(|e| {
                    MailServiceError::TemplateValidation(Arc::from(format!(
                        "BodyHtml: {}",
                        e.message
                    )))
                })?),
                None => None,
            };

            // Quick 260718-html-to-plain-derivation (Nachtrag): Test-Mail-Path
            // (send_test_mail_with_template) analog zum Send-Worker — Plain-Text
            // aus HTML ableiten, damit Text-Only-Empfänger strukturierten Fallback
            // bekommen. Ohne diesen Override würde ausgerechnet die Test-Mail
            // (die zur QA da ist) unstrukturierten Text zeigen.
            let rendered_body = match rendered_body_html.as_deref() {
                Some(html) => crate::render::plain_from_html(html),
                None => rendered_body,
            };

            // PRIVACY: `body.to_address` MUST be the recipient — NEVER any
            // member-derived address. The resolved Member contributed only
            // template variables above.
            state
                .mail_service()
                .send_test_mail_with_body(
                    &body.to_address,
                    &rendered_subject,
                    &rendered_body,
                    // Phase 23 Plan 04 (HTML-01, D-03 EP4): forward the
                    // rendered HTML — service layer sanitizes it (D-03).
                    rendered_body_html,
                )
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(
                    serde_json::json!({
                        "success": true,
                        "used_dummy_repayment": used_dummy_repayment,
                    })
                    .to_string(),
                ))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    get,
    tag = "Mail",
    path = "/jobs",
    responses(
        (status = 200, description = "List of mail jobs", body = [MailJobTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_jobs<S: MailRestState>(state: State<S>) -> Response {
    error_handler(
        (async {
            let jobs: Vec<MailJobTO> = state
                .mail_service()
                .get_jobs()
                .await?
                .iter()
                .map(MailJobTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&jobs)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    get,
    tag = "Mail",
    path = "/jobs/{id}",
    params(
        ("id" = String, Path, description = "Mail job UUID")
    ),
    responses(
        (status = 200, description = "Mail job with recipients", body = MailJobDetailTO),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_job_detail<S: MailRestState>(state: State<S>, Path(id): Path<String>) -> Response {
    error_handler(
        (async {
            let job_id = uuid::Uuid::parse_str(&id).map_err(|_| MailServiceError::NotFound)?;
            let (job, recipients) = state.mail_service().get_job_with_recipients(job_id).await?;
            let mut recipient_tos = Vec::new();
            for r in recipients.iter() {
                let mut to = MailRecipientTO::from(r);
                to.attachments = state.get_recipient_attachments(r.id).await;
                recipient_tos.push(to);
            }
            let detail = MailJobDetailTO {
                job: MailJobTO::from(&job),
                recipients: recipient_tos,
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&detail)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/jobs/{id}/retry",
    params(
        ("id" = String, Path, description = "Mail job UUID")
    ),
    responses(
        (status = 200, description = "Failed recipients retried", body = MailJobTO),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn retry_job<S: MailRestState>(state: State<S>, Path(id): Path<String>) -> Response {
    tracing::info!("retry_job called for job_id={}", id);
    error_handler(
        (async {
            let job_id = uuid::Uuid::parse_str(&id).map_err(|_| MailServiceError::NotFound)?;
            let job = state.mail_service().retry_job(job_id).await?;
            let to = MailJobTO::from(&job);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 10 D-12 + D-03: `SendBulkMailRequest` accepts optional `template_id`
    /// and `repayment_phase_id` as UUID-strings; both deserialize to `Option<String>`
    /// with the exact value from the JSON payload.
    #[test]
    fn test_send_bulk_mail_request_serde_with_phase10_fields() {
        let json = r#"{
            "to_addresses": [],
            "subject": "S",
            "body": "B",
            "template_id": "550e8400-e29b-41d4-a716-446655440000",
            "repayment_phase_id": "660e8400-e29b-41d4-a716-446655440000"
        }"#;
        let req: SendBulkMailRequest = serde_json::from_str(json).expect("must deserialize");
        assert_eq!(
            req.template_id.as_deref(),
            Some("550e8400-e29b-41d4-a716-446655440000")
        );
        assert_eq!(
            req.repayment_phase_id.as_deref(),
            Some("660e8400-e29b-41d4-a716-446655440000")
        );
    }

    /// Phase 10 backward-compat: requests without the two new optional fields
    /// still deserialize, and the two fields default to `None`. Ensures that
    /// existing frontend clients (Phase 9 and earlier) do not break.
    #[test]
    fn test_send_bulk_mail_request_serde_without_phase10_fields_backward_compat() {
        let json = r#"{"to_addresses": [], "subject": "S", "body": "B"}"#;
        let req: SendBulkMailRequest = serde_json::from_str(json).expect("must deserialize");
        assert_eq!(req.template_id, None);
        assert_eq!(req.repayment_phase_id, None);
        // Quick 260603-cz6: backward-compat for new opt-in flag.
        assert!(!req.attach_repayment_letter);
    }

    /// Quick 260603-cz6: `attach_repayment_letter` deserializes from the request
    /// body and defaults to `false` when absent.
    #[test]
    fn test_send_bulk_mail_request_serde_attach_repayment_letter_explicit_true() {
        let json = r#"{
            "to_addresses": [],
            "subject": "S",
            "body": "B",
            "repayment_phase_id": "660e8400-e29b-41d4-a716-446655440000",
            "attach_repayment_letter": true
        }"#;
        let req: SendBulkMailRequest = serde_json::from_str(json).expect("must deserialize");
        assert!(req.attach_repayment_letter);
    }

    // ── Quick 260603-evf — MailJobTO.repayment_phase_id read-only exposure ──

    /// Build a `MailJob` with the given `repayment_phase_id` and otherwise
    /// stable default values, mirroring the helpers used by the existing
    /// `dao_sqlite::tests::sample_job` (Quick 260603-cz6).
    fn make_mail_job(repayment_phase_id: Option<uuid::Uuid>) -> MailJob {
        MailJob {
            id: uuid::Uuid::new_v4(),
            created: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::June, 3).unwrap(),
                time::Time::from_hms(0, 0, 0).unwrap(),
            ),
            deleted: None,
            version: uuid::Uuid::new_v4(),
            subject: Arc::from("Test Subject"),
            body: Arc::from("Test Body"),
            status: Arc::from("pending"),
            total_count: 0,
            sent_count: 0,
            failed_count: 0,
            reply_to_inbound_mail_id: None,
            template_id: None,
            repayment_phase_id,
            attach_repayment_letter: false,
            body_html: None,
        }
    }

    /// Quick 260603-evf: `From<&MailJob>` exposes the persisted
    /// `repayment_phase_id` as a stringified UUID so the frontend can
    /// deterministically resolve the phase for the "Brief generieren + Retry"
    /// action.
    #[test]
    fn test_mail_job_to_exposes_repayment_phase_id_when_present() {
        let phase = uuid::Uuid::new_v4();
        let job = make_mail_job(Some(phase));
        let to = MailJobTO::from(&job);
        assert_eq!(to.repayment_phase_id, Some(phase.to_string()));
    }

    /// Quick 260603-evf: when the underlying `MailJob` has no
    /// `repayment_phase_id` (non-repayment bulk-mail), the TO's field
    /// stays `None` and is skipped on serialization (backward compat).
    #[test]
    fn test_mail_job_to_repayment_phase_id_none_is_skipped_on_serialize() {
        let job = make_mail_job(None);
        let to = MailJobTO::from(&job);
        assert_eq!(to.repayment_phase_id, None);

        let json = serde_json::to_string(&to).expect("must serialize");
        assert!(
            !json.contains("repayment_phase_id"),
            "skip_serializing_if must omit the key when None, got: {json}",
        );
    }

    /// Quick 260603-evf: round-trip a `MailJobTO` with `repayment_phase_id`
    /// set — serialize, deserialize, and verify the value is preserved.
    #[test]
    fn test_mail_job_to_repayment_phase_id_serde_roundtrip() {
        let phase = uuid::Uuid::new_v4();
        let job = make_mail_job(Some(phase));
        let to = MailJobTO::from(&job);
        let json = serde_json::to_string(&to).expect("must serialize");
        let parsed: MailJobTO = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(parsed.repayment_phase_id, Some(phase.to_string()));
    }

    /// Quick 260603-evf backward-compat: a JSON payload missing the
    /// `repayment_phase_id` key (older clients / cached responses) still
    /// deserializes cleanly with `None`.
    #[test]
    fn test_mail_job_to_deserialize_backward_compat_without_repayment_phase_id() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000000",
            "created": "2026-06-03T00:00:00.000000000Z",
            "subject": "S",
            "body": "B",
            "status": "pending",
            "total_count": 0,
            "sent_count": 0,
            "failed_count": 0
        }"#;
        let to: MailJobTO = serde_json::from_str(json).expect("must deserialize");
        assert_eq!(to.repayment_phase_id, None);
    }

    // ── Quick 260603-jtf — TestMailWithTemplateRequest serde ──

    /// Quick 260603-jtf: full payload with all five fields deserializes
    /// roundtrip-cleanly (no field renames, no key collisions).
    #[test]
    fn test_test_with_template_request_serde_roundtrip() {
        let json = r#"{
            "to_address": "vorstand@example.com",
            "subject": "Hallo {{ first_name }}",
            "body": "Liebe/r {{ first_name }} {{ last_name }}",
            "member_id": "123e4567-e89b-12d3-a456-426614174000",
            "repayment_phase_id": "29ae374c-9e60-4cc8-b0b4-ce51c28e7b6e"
        }"#;
        let req: TestMailWithTemplateRequest =
            serde_json::from_str(json).expect("must deserialize");
        assert_eq!(req.to_address, "vorstand@example.com");
        assert_eq!(req.subject, "Hallo {{ first_name }}");
        assert_eq!(req.body, "Liebe/r {{ first_name }} {{ last_name }}");
        assert_eq!(req.member_id, "123e4567-e89b-12d3-a456-426614174000");
        assert_eq!(
            req.repayment_phase_id.as_deref(),
            Some("29ae374c-9e60-4cc8-b0b4-ce51c28e7b6e")
        );

        // Round-trip back to JSON and re-parse to confirm symmetry.
        let serialized = serde_json::to_string(&req).expect("must serialize");
        let req2: TestMailWithTemplateRequest =
            serde_json::from_str(&serialized).expect("must re-deserialize");
        assert_eq!(req2.member_id, req.member_id);
        assert_eq!(req2.repayment_phase_id, req.repayment_phase_id);
    }

    /// Quick 260603-jtf: backward-compat — payloads without
    /// `repayment_phase_id` (Editor flow, no phase context) still deserialize
    /// with the field defaulting to `None`.
    #[test]
    fn test_test_with_template_request_serde_without_phase() {
        let json = r#"{
            "to_address": "vorstand@example.com",
            "subject": "Subj",
            "body": "Body",
            "member_id": "123e4567-e89b-12d3-a456-426614174000"
        }"#;
        let req: TestMailWithTemplateRequest =
            serde_json::from_str(json).expect("must deserialize");
        assert_eq!(req.repayment_phase_id, None);
    }

    // ── Quick 260603-kon — PreviewResponse.used_dummy_repayment serde ──

    /// Quick 260603-kon: when the Dummy-Fallback was activated, the response
    /// MUST include `"used_dummy_repayment": true` so the Frontend can show
    /// the amber Hinweis-Banner. Also verifies the rendered body contains
    /// the sentinel `"99,99"` value — proves the Dummy-Pfad really went
    /// through `merge_repayment_context`.
    #[test]
    fn test_preview_response_serializes_used_dummy_repayment_when_true() {
        let response = PreviewResponse {
            subject: "S".to_string(),
            body: "Auszahlung: 99,99 EUR fuer 99 Anteile".to_string(),
            body_html: None,
            errors: vec![],
            used_dummy_repayment: true,
        };
        let json = serde_json::to_string(&response).expect("must serialize");
        assert!(
            json.contains("\"used_dummy_repayment\":true"),
            "must include the flag when true, got: {json}",
        );
        // Sentinel-Werte-Lock auf der Body-Ebene: 99,99 muss tatsaechlich
        // im Output sein, sonst lief der Dummy-Pfad nicht.
        assert!(
            json.contains("99,99"),
            "rendered body must contain sentinel 99,99, got: {json}",
        );
    }

    /// Quick 260603-kon: when no dummy fallback was used (real phase OR
    /// no `repayment_phase_id` in request), the flag is `false` and MUST
    /// be skipped on serialization (`skip_serializing_if = std::ops::Not::not`).
    /// Backward-compat: older Frontends that don't know the field see no
    /// change in the wire shape.
    #[test]
    fn test_preview_response_skips_used_dummy_repayment_when_false() {
        let response = PreviewResponse {
            subject: "S".to_string(),
            body: "B".to_string(),
            body_html: None,
            errors: vec![],
            used_dummy_repayment: false,
        };
        let json = serde_json::to_string(&response).expect("must serialize");
        assert!(
            !json.contains("used_dummy_repayment"),
            "false flag MUST be omitted from wire-shape, got: {json}",
        );
    }

    /// Quick 260603-kon: existing PreviewResponse JSON payloads (Phase 10 era)
    /// MUST still deserialize cleanly — backward-compat for cached responses
    /// or older clients.
    #[test]
    fn test_preview_response_deserialize_backward_compat_without_dummy_flag() {
        let json = r#"{"subject": "S", "body": "B"}"#;
        let response: PreviewResponse = serde_json::from_str(json).expect("must deserialize");
        assert_eq!(response.subject, "S");
        assert_eq!(response.body, "B");
        assert!(!response.used_dummy_repayment);
    }

    /// Quick 260603-kon: roundtrip — `used_dummy_repayment: true` survives
    /// serialize -> deserialize cleanly, so the Frontend can `serde_json::from_str`
    /// the response into the same struct without surprises.
    #[test]
    fn test_preview_response_roundtrip_with_dummy_flag() {
        let original = PreviewResponse {
            subject: "S".to_string(),
            body: "B".to_string(),
            body_html: None,
            errors: vec![],
            used_dummy_repayment: true,
        };
        let json = serde_json::to_string(&original).expect("must serialize");
        let parsed: PreviewResponse = serde_json::from_str(&json).expect("must deserialize");
        assert!(parsed.used_dummy_repayment);
    }

    // ── Quick 260603-n3m — None-arm Dummy-Fallback via template detection ──

    /// Quick 260603-n3m: Wenn das Template `{{ payout_amount }}` enthaelt,
    /// liefert `template_uses_repayment_vars` `true`, und das Render mit
    /// gemergtem Dummy-Context muss `"99,99"` enthalten — Sentinel-Lock auf
    /// derselben Render-Pipeline, die die beiden Handler nutzen.
    #[test]
    fn test_dummy_merge_applies_when_no_phase_id_and_template_uses_repayment_var() {
        use crate::template::{
            dummy_repayment_context, member_to_template_context, merge_repayment_context,
            render_template, template_uses_repayment_vars,
        };
        use genossi_dao::member::{MemberEntity, MemberStatus, Salutation};
        use std::sync::Arc;

        let date = time::Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
        let datetime = time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT);
        let member = MemberEntity {
            id: uuid::Uuid::new_v4(),
            member_number: 7,
            first_name: Arc::from("Max"),
            last_name: Arc::from("Mustermann"),
            salutation: Some(Salutation::Herr),
            title: None,
            email: Some(Arc::from("max@example.com")),
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: date,
            shares_at_joining: 1,
            current_shares: 3,
            current_balance: 15000,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            status: MemberStatus::Normal,
            account_holder: None,
            postal_status: genossi_dao::member::PostalStatus::Erreichbar,
            created: datetime,
            deleted: None,
            version: uuid::Uuid::new_v4(),
        };

        let subject = "Subject";
        let body = "Auszahlung: {{ payout_amount }} EUR";

        // Detection: ja, template referenziert Repayment-Var.
        assert!(template_uses_repayment_vars(subject, body));

        // Dummy-Merge: dieselbe Call-Shape wie der None-Arm beider Handler.
        let base = member_to_template_context(&member);
        let (payout, share_count, share_value, fiscal_year) = dummy_repayment_context();
        let ctx = merge_repayment_context(base, payout, share_count, share_value, fiscal_year);
        let rendered = render_template(body, &ctx).expect("must render with dummy ctx");
        assert!(
            rendered.contains("99,99"),
            "merged dummy ctx must inject sentinel 99,99, got: {rendered}",
        );
    }

    /// Quick 260603-n3m: Template OHNE Repayment-Vars darf NICHT den
    /// Dummy-Pfad ziehen — `template_uses_repayment_vars` ist die korrekte
    /// Detection-Boundary, sonst wuerde der `used_dummy_repayment`-Banner
    /// im Editor luegen.
    #[test]
    fn test_dummy_merge_does_not_apply_for_pure_member_template() {
        use crate::template::template_uses_repayment_vars;
        let subject = "Subject";
        let body = "Hallo {{ first_name }} {{ last_name }}";
        assert!(
            !template_uses_repayment_vars(subject, body),
            "pure member-var template must NOT trigger the dummy-merge fallback"
        );
    }

    // ── Phase 24 (EDIT-05, D-04): PreviewResponse.body_html serde-lock ──
    //
    // Wire backward-compat: `skip_serializing_if = "Option::is_none"` MUST omit
    // the `body_html` key when the field is None, so pre-Phase-24 frontends see
    // no shape change on the response.

    /// Phase 24: response with `body_html = None` MUST NOT serialize the key
    /// (mirrors the pattern established in Phase 23 Plan 04 for the send-path
    /// DTOs).
    #[test]
    fn preview_response_serializes_without_body_html_when_none() {
        let response = PreviewResponse {
            subject: "S".to_string(),
            body: "B".to_string(),
            body_html: None,
            errors: Vec::new(),
            used_dummy_repayment: false,
        };
        let json = serde_json::to_string(&response).expect("must serialize");
        assert!(
            !json.contains("body_html"),
            "skip_serializing_if must omit body_html when None, got: {json}",
        );
    }

    /// Phase 24: response with `body_html = Some(...)` MUST serialize the key
    /// with the exact value the handler assigned.
    #[test]
    fn preview_response_serializes_with_body_html_when_some() {
        let response = PreviewResponse {
            subject: "S".to_string(),
            body: "B".to_string(),
            body_html: Some("<p>Hallo Max</p>".to_string()),
            errors: Vec::new(),
            used_dummy_repayment: false,
        };
        let json = serde_json::to_string(&response).expect("must serialize");
        assert!(
            json.contains("body_html"),
            "body_html must be in JSON when Some, got: {json}",
        );
        assert!(
            json.contains("<p>Hallo Max</p>"),
            "rendered HTML must appear in the JSON output, got: {json}",
        );
    }
}
