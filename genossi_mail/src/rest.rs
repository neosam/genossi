use axum::body::Body;
use axum::extract::State;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};

use crate::dao::SentMail;
use crate::service::{MailService, MailServiceError};

pub trait MailRestState: Clone + Send + Sync + 'static {
    type MailService: MailService;
    fn mail_service(&self) -> Arc<Self::MailService>;
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SentMailTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: String,
    pub created: String,
    pub to_address: String,
    pub subject: String,
    pub body: String,
    #[schema(example = "sent")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SendMailRequest {
    #[schema(example = "user@example.com")]
    pub to_address: String,
    #[schema(example = "Test Subject")]
    pub subject: String,
    #[schema(example = "Hello, this is a test email.")]
    pub body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SendBulkMailRequest {
    #[schema(example = json!(["user1@example.com", "user2@example.com"]))]
    pub to_addresses: Vec<String>,
    #[schema(example = "Test Subject")]
    pub subject: String,
    #[schema(example = "Hello, this is a test email.")]
    pub body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TestMailRequest {
    #[schema(example = "admin@example.com")]
    pub to_address: String,
}

fn format_datetime(dt: &time::PrimitiveDateTime) -> String {
    dt.assume_utc()
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| dt.to_string())
}

impl From<&SentMail> for SentMailTO {
    fn from(mail: &SentMail) -> Self {
        Self {
            id: mail.id.to_string(),
            created: format_datetime(&mail.created),
            to_address: mail.to_address.to_string(),
            subject: mail.subject.to_string(),
            body: mail.body.to_string(),
            status: mail.status.to_string(),
            error: mail.error.as_deref().map(String::from),
            sent_at: mail.sent_at.as_ref().map(format_datetime),
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
        .route("/test", post(send_test_mail::<S>))
        .route("/sent", get(get_sent_mails::<S>))
}

#[derive(OpenApi)]
#[openapi(
    paths(send_mail, send_bulk_mail, send_test_mail, get_sent_mails),
    components(schemas(SentMailTO, SendMailRequest, SendBulkMailRequest, TestMailRequest)),
    tags((name = "Mail", description = "Email sending and history endpoints"))
)]
pub struct ApiDoc;

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail",
    path = "/send",
    request_body = SendMailRequest,
    responses(
        (status = 200, description = "Mail sent (or failed with error details)", body = SentMailTO),
        (status = 400, description = "SMTP config missing"),
        (status = 422, description = "Invalid request"),
        (status = 502, description = "SMTP error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn send_mail<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<SendMailRequest>,
) -> Response {
    error_handler(
        (async {
            let result = state
                .mail_service()
                .send_mail(&body.to_address, &body.subject, &body.body)
                .await?;
            let to = SentMailTO::from(&result);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
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
        (status = 200, description = "Mails sent (each with individual status)", body = [SentMailTO]),
        (status = 400, description = "SMTP config missing"),
        (status = 422, description = "Invalid request"),
        (status = 502, description = "SMTP error"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn send_bulk_mail<S: MailRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<SendBulkMailRequest>,
) -> Response {
    error_handler(
        (async {
            let results = state
                .mail_service()
                .send_mails(&body.to_addresses, &body.subject, &body.body)
                .await?;
            let tos: Vec<SentMailTO> = results.iter().map(SentMailTO::from).collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&tos).unwrap()))
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
        (status = 200, description = "Test mail sent (or failed with error details)", body = SentMailTO),
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
            let result = state
                .mail_service()
                .send_test_mail(&body.to_address)
                .await?;
            let to = SentMailTO::from(&result);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    get,
    tag = "Mail",
    path = "/sent",
    responses(
        (status = 200, description = "List of sent mails", body = [SentMailTO]),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_sent_mails<S: MailRestState>(state: State<S>) -> Response {
    error_handler(
        (async {
            let mails: Vec<SentMailTO> = state
                .mail_service()
                .get_sent_mails()
                .await?
                .iter()
                .map(SentMailTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&mails).unwrap()))
                .unwrap())
        })
        .await,
    )
}
