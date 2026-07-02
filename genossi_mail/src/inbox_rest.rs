//! REST layer for the member inbox. Mirrors the layout of `rest.rs` for the
//! outbound side: a `InboxRestState` trait plus a `router()` function.

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::dao::{InboundMail, InboundMailAttachment};
use crate::inbox::InboxService;
use crate::service::MailServiceError;
use genossi_service::document_storage::{DocumentStorage, StorageError};

// ────────────────────────────────────────────────────────────────────────────
// Transport objects
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundMailTO {
    pub id: String,
    pub from_address: String,
    pub subject: String,
    pub received_at: String,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub replied: bool,
    pub done: bool,
    pub archived: bool,
    pub assigned_member_id: Option<String>,
    pub assigned_member_name: Option<String>,
}

/// Phase 19 D-07: Attachment metadata embedded in detail responses + returned
/// by download-handler error responses. `oversized=true` rows have empty
/// bytes on disk; clients render them as "rejected at receive" markers.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundMailAttachmentTO {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub oversized: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundMailDetailTO {
    pub id: String,
    pub from_address: String,
    pub subject: String,
    pub received_at: String,
    pub body_text: String,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub replied: bool,
    pub done: bool,
    pub archived: bool,
    pub assigned_member_id: Option<String>,
    pub assigned_member_name: Option<String>,
    /// Phase 19 D-07: populated by `InboxService::list_attachments`.
    pub attachments: Vec<InboundMailAttachmentTO>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssignMemberRequest {
    pub member_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplyRequest {
    pub subject: String,
    pub body: String,
    /// Quick 260607-s0s: optional MemberDocument IDs (UUIDs as strings) to be
    /// attached to the reply, mirroring the Compose-flow picker. Defaults to
    /// empty for backward compatibility with older frontends.
    #[serde(default)]
    pub attachment_ids: Vec<String>,
    /// Quick 260607-s0s: optional StaticDocument IDs (UUIDs as strings) to be
    /// attached job-level. Defaults to empty for backward compatibility.
    #[serde(default)]
    pub static_document_ids: Vec<String>,
    /// Phase 24 (EDIT-01, EDIT-03, D-01): optional HTML sibling of `body`.
    /// When present, sanitized once at the store boundary (Phase 23 D-03 EP
    /// wire — `sanitize_body_html_opt`) and persisted on the MailJob so the
    /// worker sends a multipart/alternative reply. `None` ⇒ text-only reply
    /// (backward-compat with pre-Phase-24 frontends).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_html: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplyResponseTO {
    pub job_id: String,
    pub status: String,
}

fn format_dt(dt: &time::PrimitiveDateTime) -> String {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc().format(format).unwrap_or_default()
}

fn to_list_to(mail: &InboundMail, assigned_name: Option<String>) -> InboundMailTO {
    InboundMailTO {
        id: mail.id.to_string(),
        from_address: mail.from_address.to_string(),
        subject: mail.subject.to_string(),
        received_at: format_dt(&mail.received_at),
        has_attachments: mail.has_attachments,
        has_html_body: mail.has_html_body,
        replied: mail.replied,
        done: mail.done,
        archived: mail.archived,
        assigned_member_id: mail.assigned_member_id.map(|id| id.to_string()),
        assigned_member_name: assigned_name,
    }
}

fn to_attachment_to(a: &InboundMailAttachment) -> InboundMailAttachmentTO {
    InboundMailAttachmentTO {
        id: a.id.to_string(),
        file_name: a.file_name.to_string(),
        mime_type: a.mime_type.to_string(),
        size_bytes: a.size_bytes,
        oversized: a.oversized,
    }
}

fn to_detail_to(
    mail: &InboundMail,
    assigned_name: Option<String>,
    attachments: Vec<InboundMailAttachmentTO>,
) -> InboundMailDetailTO {
    InboundMailDetailTO {
        id: mail.id.to_string(),
        from_address: mail.from_address.to_string(),
        subject: mail.subject.to_string(),
        received_at: format_dt(&mail.received_at),
        body_text: mail.body_text.to_string(),
        has_attachments: mail.has_attachments,
        has_html_body: mail.has_html_body,
        replied: mail.replied,
        done: mail.done,
        archived: mail.archived,
        assigned_member_id: mail.assigned_member_id.map(|id| id.to_string()),
        assigned_member_name: assigned_name,
        attachments,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// State trait
// ────────────────────────────────────────────────────────────────────────────

pub trait InboxRestState: Clone + Send + Sync + 'static {
    type InboxService: InboxService;
    fn inbox_service(&self) -> Arc<Self::InboxService>;
    /// Resolve a member's display name (first last) by id, for list labels.
    fn resolve_member_name(
        &self,
        member_id: Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>>;

    /// Phase 19 D-08: storage handle used by the attachment download
    /// endpoint to load attachment bytes. Returned via trait method so
    /// `genossi_mail` does not depend on the concrete storage impl.
    /// Named `inbox_document_storage` (not `document_storage`) to avoid
    /// a method-resolution clash with `RestStateDef::document_storage`
    /// — both traits are implemented on the same `RestStateImpl` and
    /// rustc rejects same-named accessors as ambiguous.
    fn inbox_document_storage(&self) -> Arc<dyn DocumentStorage>;

    /// Phase 19 (T-02): build a `Content-Disposition` header value. These
    /// two accessors exist so the download-handler in `genossi_mail` can
    /// build the header WITHOUT importing `genossi_rest` (which would
    /// create a circular crate dependency — `genossi_rest` already
    /// depends on `genossi_mail`). `genossi_bin` (the only crate that
    /// imports both) implements them by delegating to
    /// `genossi_rest::http_util::content_disposition_*`.
    fn content_disposition_attachment(&self, filename: &str) -> String;
    fn content_disposition_inline(&self, filename: &str) -> String;

    /// Quick 260607-s0s: resolve a MemberDocument by id so the reply handler
    /// can validate ownership (attachment_id must belong to the inbox-mail's
    /// assigned_member_id). Identical signature to
    /// [`crate::rest::MailRestState::resolve_document`] — both implemented
    /// on the same `RestStateImpl` in `genossi_bin`.
    fn resolve_document(
        &self,
        document_id: Uuid,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Option<crate::rest::ResolvedDocument>> + Send + '_>,
    >;
}

// ────────────────────────────────────────────────────────────────────────────
// Error mapping
// ────────────────────────────────────────────────────────────────────────────

fn map_error(e: MailServiceError) -> Response {
    let (code, msg) = match e {
        MailServiceError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
        MailServiceError::ConfigMissing(k) => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("missing config: {}", k),
        ),
        MailServiceError::SmtpError(m) => (StatusCode::BAD_GATEWAY, m.to_string()),
        MailServiceError::DataAccess(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.to_string()),
        MailServiceError::TemplateValidation(m) => (StatusCode::BAD_REQUEST, m.to_string()),
        MailServiceError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.to_string()),
    };
    (code, msg).into_response()
}

// ────────────────────────────────────────────────────────────────────────────
// Handlers
// ────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/",
    tag = "inbox",
    responses(
        (status = 200, description = "List inbound mails", body = [InboundMailTO])
    )
)]
async fn list_inbox<S: InboxRestState>(State(state): State<S>) -> Response {
    let svc = state.inbox_service();
    let mails = match svc.list().await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };
    let mut out: Vec<InboundMailTO> = Vec::with_capacity(mails.len());
    for m in mails.iter() {
        let name = match m.assigned_member_id {
            Some(id) => state.resolve_member_name(id).await,
            None => None,
        };
        out.push(to_list_to(m, name));
    }
    Json(out).into_response()
}

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "inbox",
    params(("id" = String, Path, description = "Inbound mail id")),
    responses(
        (status = 200, description = "Inbound mail detail", body = InboundMailDetailTO),
        (status = 404, description = "Not found")
    )
)]
async fn get_inbox<S: InboxRestState>(State(state): State<S>, Path(id): Path<String>) -> Response {
    let svc = state.inbox_service();
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };
    let mail = match svc.get(uuid).await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };
    // Phase 19 D-07: embed attachment metadata in detail responses.
    let attachments = match svc.list_attachments(uuid).await {
        Ok(a) => a,
        Err(e) => return map_error(e),
    };
    let attachment_tos: Vec<InboundMailAttachmentTO> =
        attachments.iter().map(to_attachment_to).collect();
    let name = match mail.assigned_member_id {
        Some(id) => state.resolve_member_name(id).await,
        None => None,
    };
    Json(to_detail_to(&mail, name, attachment_tos)).into_response()
}

#[utoipa::path(
    post,
    path = "/{id}/assign",
    tag = "inbox",
    request_body = AssignMemberRequest,
    params(("id" = String, Path, description = "Inbound mail id")),
    responses(
        (status = 200, description = "Assigned", body = InboundMailTO)
    )
)]
async fn assign_inbox<S: InboxRestState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<AssignMemberRequest>,
) -> Response {
    let svc = state.inbox_service();
    let mail_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };
    let member_id = match Uuid::parse_str(&req.member_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid member_id").into_response(),
    };
    let mail = match svc.assign_member(mail_id, member_id).await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };
    let name = state.resolve_member_name(member_id).await;
    Json(to_list_to(&mail, name)).into_response()
}

#[utoipa::path(
    post,
    path = "/{id}/unassign",
    tag = "inbox",
    params(("id" = String, Path, description = "Inbound mail id")),
    responses((status = 200, description = "Unassigned", body = InboundMailTO))
)]
async fn unassign_inbox<S: InboxRestState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Response {
    let svc = state.inbox_service();
    let mail_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };
    let mail = match svc.unassign(mail_id).await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };
    Json(to_list_to(&mail, None)).into_response()
}

#[utoipa::path(
    post,
    path = "/{id}/mark-read",
    tag = "inbox",
    params(("id" = String, Path, description = "Inbound mail id")),
    responses((status = 200, description = "Marked as read"))
)]
async fn mark_read_inbox<S: InboxRestState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Response {
    let svc = state.inbox_service();
    let mail_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };
    match svc.mark_read(mail_id).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => map_error(e),
    }
}

#[utoipa::path(
    post,
    path = "/{id}/archive",
    tag = "inbox",
    params(("id" = String, Path, description = "Inbound mail id")),
    responses((status = 200, description = "Archived", body = InboundMailTO))
)]
async fn archive_inbox<S: InboxRestState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Response {
    let svc = state.inbox_service();
    let mail_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };
    let mail = match svc.archive(mail_id).await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };
    let name = match mail.assigned_member_id {
        Some(id) => state.resolve_member_name(id).await,
        None => None,
    };
    Json(to_list_to(&mail, name)).into_response()
}

#[utoipa::path(
    post,
    path = "/{id}/done",
    tag = "inbox",
    params(("id" = String, Path, description = "Inbound mail id")),
    responses((status = 200, description = "Marked as done", body = InboundMailTO))
)]
async fn done_inbox<S: InboxRestState>(State(state): State<S>, Path(id): Path<String>) -> Response {
    let svc = state.inbox_service();
    let mail_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };
    let mail = match svc.mark_done(mail_id).await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };
    let name = match mail.assigned_member_id {
        Some(id) => state.resolve_member_name(id).await,
        None => None,
    };
    Json(to_list_to(&mail, name)).into_response()
}

#[utoipa::path(
    post,
    path = "/{id}/reply",
    tag = "inbox",
    request_body = ReplyRequest,
    params(("id" = String, Path, description = "Inbound mail id")),
    responses(
        (status = 202, description = "Reply job created", body = ReplyResponseTO),
        (status = 404, description = "Inbound mail not found")
    )
)]
async fn reply_inbox<S: InboxRestState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<ReplyRequest>,
) -> Response {
    let svc = state.inbox_service();
    let mail_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid id").into_response(),
    };

    // Quick 260607-s0s: load the mail up front so we can validate ownership
    // (attachment_id must belong to mail.assigned_member_id). Mirrors the
    // Compose-flow validation in rest.rs:481-513.
    let mail = match svc.get(mail_id).await {
        Ok(m) => m,
        Err(e) => return map_error(e),
    };

    // T-s0s-02: attaching MemberDocuments requires an assigned member —
    // otherwise we cannot perform the ownership check.
    if !req.attachment_ids.is_empty() && mail.assigned_member_id.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "no member assigned to this mail — cannot attach member documents",
        )
            .into_response();
    }

    // T-s0s-01: resolve each attachment_id and verify the document belongs
    // to the assigned member. Reject mismatches with 400 BadRequest.
    let mut attachment_inputs: Vec<crate::service::AttachmentInput> = Vec::new();
    for att_id_str in &req.attachment_ids {
        let doc_id = match Uuid::parse_str(att_id_str) {
            Ok(u) => u,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("invalid attachment_id: {}", att_id_str),
                )
                    .into_response();
            }
        };
        let doc = match state.resolve_document(doc_id).await {
            Some(d) => d,
            None => return (StatusCode::NOT_FOUND, "attachment not found").into_response(),
        };
        if Some(doc.member_id) != mail.assigned_member_id {
            return (
                StatusCode::BAD_REQUEST,
                "Attachment does not belong to the recipient's member",
            )
                .into_response();
        }
        attachment_inputs.push(crate::service::AttachmentInput {
            document_id: doc.document_id,
            file_name: doc.file_name,
            mime_type: doc.mime_type,
            relative_path: doc.relative_path,
        });
    }

    // T-s0s-03: parse static_document_ids — invalid UUIDs are a client error
    // (400). Existence is enforced inside the service (mirrors
    // MailServiceImpl::create_job static-doc validation).
    let mut static_doc_uuids: Vec<Uuid> = Vec::new();
    for sid in &req.static_document_ids {
        let parsed = match Uuid::parse_str(sid) {
            Ok(u) => u,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("invalid static_document_id: {}", sid),
                )
                    .into_response();
            }
        };
        static_doc_uuids.push(parsed);
    }

    match svc
        .reply(
            mail_id,
            &req.subject,
            &req.body,
            attachment_inputs,
            static_doc_uuids,
            // Phase 24 (EDIT-01, D-01): pass through the optional HTML sibling
            // — the service sanitizes it at the store boundary.
            req.body_html.clone(),
        )
        .await
    {
        Ok(job) => (
            StatusCode::ACCEPTED,
            Json(ReplyResponseTO {
                job_id: job.id.to_string(),
                status: job.status.to_string(),
            }),
        )
            .into_response(),
        Err(e) => map_error(e),
    }
}

#[utoipa::path(
    get,
    path = "/folders",
    tag = "inbox",
    responses(
        (status = 200, description = "List IMAP folder names", body = [String]),
        (status = 503, description = "IMAP not configured or unreachable")
    )
)]
async fn list_folders<S: InboxRestState>(State(state): State<S>) -> Response {
    let svc = state.inbox_service();
    match svc.list_folders().await {
        Ok(folders) => Json(folders).into_response(),
        Err(e) => map_error(e),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Phase 19: Attachment download endpoint
// ────────────────────────────────────────────────────────────────────────────

/// Query parameter for the attachment download endpoint.
/// `disposition=inline` switches the `Content-Disposition` to `inline; …`;
/// anything else (or missing) defaults to `attachment; …` (D-08).
#[derive(Deserialize)]
pub struct DispositionQuery {
    pub disposition: Option<String>,
}

#[utoipa::path(
    get,
    path = "/{mail_id}/attachments/{attachment_id}",
    tag = "inbox",
    params(
        ("mail_id" = String, Path, description = "Inbound mail id"),
        ("attachment_id" = String, Path, description = "Attachment id"),
        ("disposition" = Option<String>, Query, description = "inline | attachment (default attachment)"),
    ),
    responses(
        (status = 200, description = "Binary attachment bytes"),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Not found (mail/attachment/file)"),
        (status = 410, description = "Attachment was rejected as oversized at receive"),
    ),
)]
async fn download_attachment<S: InboxRestState>(
    State(state): State<S>,
    Path((mail_id, attachment_id)): Path<(String, String)>,
    Query(q): Query<DispositionQuery>,
) -> Response {
    let mail_uuid = match Uuid::parse_str(&mail_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid mail_id").into_response(),
    };
    let att_uuid = match Uuid::parse_str(&attachment_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid attachment_id").into_response(),
    };
    // T-03 IDOR mitigation: service.find_attachment delegates to DAO
    // `find_by_id_and_mail` which requires both keys to match.
    let att = match state
        .inbox_service()
        .find_attachment(mail_uuid, att_uuid)
        .await
    {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, "attachment not found").into_response(),
        Err(e) => return map_error(e),
    };
    // D-02 + UI-Feedback: oversized rows have no file on disk. 410 GONE is
    // semantically distinct from 404 (the row exists, the bytes were
    // rejected at receive).
    if att.oversized || att.relative_path.is_none() {
        return (
            StatusCode::GONE,
            "attachment was rejected for size at receive",
        )
            .into_response();
    }
    let rel_path = att
        .relative_path
        .as_ref()
        .expect("relative_path is Some — checked above")
        .to_string();
    let bytes = match state.inbox_document_storage().load(&rel_path).await {
        Ok(b) => b,
        Err(StorageError::NotFound) => {
            return (StatusCode::NOT_FOUND, "file not found").into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("storage: {}", e),
            )
                .into_response()
        }
    };
    // T-02: Content-Disposition filename goes through http_util helpers
    // (RFC 6266 + CR/LF strip + UTF-8 percent-encoding) via trait accessors.
    let header = match q.disposition.as_deref() {
        Some("inline") => state.content_disposition_inline(&att.file_name),
        _ => state.content_disposition_attachment(&att.file_name),
    };
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", att.mime_type.as_ref())
        .header("Content-Disposition", header)
        .body(Body::from(bytes))
        .unwrap()
}

pub fn generate_route<S: InboxRestState>() -> Router<S> {
    Router::new()
        .route("/", get(list_inbox::<S>))
        .route("/folders", get(list_folders::<S>))
        .route("/{id}", get(get_inbox::<S>))
        .route("/{id}/assign", post(assign_inbox::<S>))
        .route("/{id}/unassign", post(unassign_inbox::<S>))
        .route("/{id}/mark-read", post(mark_read_inbox::<S>))
        .route("/{id}/archive", post(archive_inbox::<S>))
        .route("/{id}/done", post(done_inbox::<S>))
        .route("/{id}/reply", post(reply_inbox::<S>))
        // Phase 19: attachment download endpoint (Vorstand-only via the
        // existing /api/inbox/* auth middleware that already protects
        // GET /api/inbox/{id} — no new permission code, D-09 + T-04).
        .route(
            "/{mail_id}/attachments/{attachment_id}",
            get(download_attachment::<S>),
        )
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_inbox,
        list_folders,
        get_inbox,
        assign_inbox,
        unassign_inbox,
        mark_read_inbox,
        archive_inbox,
        done_inbox,
        reply_inbox,
        download_attachment,
    ),
    components(schemas(InboundMailTO, InboundMailDetailTO, InboundMailAttachmentTO, AssignMemberRequest, ReplyRequest, ReplyResponseTO)),
    tags((name = "inbox", description = "Member inbox (incoming mails)"))
)]
pub struct InboxApiDoc;
