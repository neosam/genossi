//! REST layer for the member inbox. Mirrors the layout of `rest.rs` for the
//! outbound side: a `InboxRestState` trait plus a `router()` function.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::dao::InboundMail;
use crate::inbox::InboxService;
use crate::service::MailServiceError;

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
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssignMemberRequest {
    pub member_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplyRequest {
    pub subject: String,
    pub body: String,
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

fn to_detail_to(mail: &InboundMail, assigned_name: Option<String>) -> InboundMailDetailTO {
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
    let name = match mail.assigned_member_id {
        Some(id) => state.resolve_member_name(id).await,
        None => None,
    };
    Json(to_detail_to(&mail, name)).into_response()
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
    match svc.reply(mail_id, &req.subject, &req.body).await {
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
    ),
    components(schemas(InboundMailTO, InboundMailDetailTO, AssignMemberRequest, ReplyRequest, ReplyResponseTO)),
    tags((name = "inbox", description = "Member inbox (incoming mails)"))
)]
pub struct InboxApiDoc;
