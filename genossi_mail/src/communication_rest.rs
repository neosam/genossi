//! REST layer for the member communication timeline.
//! Provides a unified, chronological view of inbound and outbound mails per member.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::dao::{CommunicationDao, CommunicationDirection, CommunicationEntry};
use crate::service::MailServiceError;

// ────────────────────────────────────────────────────────────────────────────
// Transport objects
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundStatusTO {
    pub done: bool,
    pub replied: bool,
    pub archived: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CommunicationEntryTO {
    pub direction: String,
    pub date: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbox_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbound_status: Option<InboundStatusTO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound_status: Option<String>,
    // Phase 32 D-06: der bereits gespeicherte, per-Empfaenger gerenderte Body wird
    // durchgereicht (kein Re-Render). Nur bei Outbound gesetzt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_html_body: Option<String>,
}

fn format_dt(dt: &time::PrimitiveDateTime) -> String {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc().format(format).unwrap_or_default()
}

impl From<&CommunicationEntry> for CommunicationEntryTO {
    fn from(e: &CommunicationEntry) -> Self {
        let inbound_status = if e.direction == CommunicationDirection::Inbound {
            Some(InboundStatusTO {
                done: e.inbound_done.unwrap_or(false),
                replied: e.inbound_replied.unwrap_or(false),
                archived: e.inbound_archived.unwrap_or(false),
            })
        } else {
            None
        };

        CommunicationEntryTO {
            direction: match e.direction {
                CommunicationDirection::Inbound => "inbound".to_string(),
                CommunicationDirection::Outbound => "outbound".to_string(),
            },
            date: format_dt(&e.date),
            subject: e.subject.to_string(),
            inbox_id: e.inbox_id.map(|id| id.to_string()),
            from_address: e.from_address.as_ref().map(|a| a.to_string()),
            inbound_status,
            mail_job_id: e.mail_job_id.map(|id| id.to_string()),
            recipient_id: e.recipient_id.map(|id| id.to_string()),
            to_address: e.to_address.as_ref().map(|a| a.to_string()),
            outbound_status: e.outbound_status.as_ref().map(|s| s.to_string()),
            rendered_body: e.rendered_body.as_ref().map(|s| s.to_string()),
            rendered_html_body: e.rendered_html_body.as_ref().map(|s| s.to_string()),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// State trait
// ────────────────────────────────────────────────────────────────────────────

pub trait CommunicationRestState: Clone + Send + Sync + 'static {
    type CommunicationDao: CommunicationDao;
    fn communication_dao(&self) -> Arc<Self::CommunicationDao>;
}

// ────────────────────────────────────────────────────────────────────────────
// Error mapping
// ────────────────────────────────────────────────────────────────────────────

fn map_error(e: MailServiceError) -> Response {
    let (code, msg) = match e {
        MailServiceError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
        MailServiceError::DataAccess(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.to_string()),
        other => (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", other)),
    };
    (code, msg).into_response()
}

// ────────────────────────────────────────────────────────────────────────────
// Handlers
// ────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    tag = "Communication",
    path = "/",
    responses(
        (status = 200, description = "Communication timeline", body = Vec<CommunicationEntryTO>),
        (status = 400, description = "Invalid member ID"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_member_communications<S: CommunicationRestState>(
    state: State<S>,
    Path(member_id): Path<String>,
) -> Response {
    let uuid = match member_id.parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid member id").into_response(),
    };

    match state
        .communication_dao()
        .get_member_communications(uuid)
        .await
    {
        Ok(entries) => {
            let tos: Vec<CommunicationEntryTO> = entries.iter().map(Into::into).collect();
            axum::Json(tos).into_response()
        }
        Err(e) => map_error(e.into()),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Router & OpenAPI
// ────────────────────────────────────────────────────────────────────────────

pub fn generate_route<S: CommunicationRestState>() -> Router<S> {
    Router::new().route("/", get(get_member_communications::<S>))
}

#[derive(OpenApi)]
#[openapi(
    paths(get_member_communications),
    components(schemas(CommunicationEntryTO, InboundStatusTO)),
    tags((name = "Communication", description = "Member communication timeline"))
)]
pub struct CommunicationApiDoc;
