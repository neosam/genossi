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
use crate::service::{MailService, MailServiceError, RecipientInput};

pub trait MailRestState: Clone + Send + Sync + 'static {
    type MailService: MailService;
    fn mail_service(&self) -> Arc<Self::MailService>;
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MailJobTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    #[schema(example = "running")]
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
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

impl From<&MailJob> for MailJobTO {
    fn from(job: &MailJob) -> Self {
        Self {
            id: job.id.to_string(),
            created: format_datetime(&job.created),
            subject: job.subject.to_string(),
            body: job.body.to_string(),
            status: job.status.to_string(),
            total_count: job.total_count,
            sent_count: job.sent_count,
            failed_count: job.failed_count,
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
        .route("/jobs", get(get_jobs::<S>))
        .route("/jobs/{id}", get(get_job_detail::<S>))
        .route("/jobs/{id}/retry", post(retry_job::<S>))
}

#[derive(OpenApi)]
#[openapi(
    paths(send_mail, send_bulk_mail, send_test_mail, get_jobs, get_job_detail, retry_job),
    components(schemas(MailJobTO, MailRecipientTO, MailJobDetailTO, SendMailRequest, SendBulkMailRequest, BulkRecipient, TestMailRequest)),
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
                    vec![RecipientInput {
                        address: body.to_address,
                        member_id: None,
                    }],
                )
                .await?;
            let to = MailJobTO::from(&job);
            Ok(Response::builder()
                .status(202)
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
                })
                .collect();

            let job = state
                .mail_service()
                .create_job(&body.subject, &body.body, recipients)
                .await?;
            let to = MailJobTO::from(&job);
            Ok(Response::builder()
                .status(202)
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
                .body(Body::new(
                    serde_json::json!({"success": true}).to_string(),
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
                .body(Body::new(serde_json::to_string(&jobs).unwrap()))
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
pub async fn get_job_detail<S: MailRestState>(
    state: State<S>,
    Path(id): Path<String>,
) -> Response {
    error_handler(
        (async {
            let job_id = uuid::Uuid::parse_str(&id)
                .map_err(|_| MailServiceError::NotFound)?;
            let (job, recipients) = state
                .mail_service()
                .get_job_with_recipients(job_id)
                .await?;
            let detail = MailJobDetailTO {
                job: MailJobTO::from(&job),
                recipients: recipients.iter().map(MailRecipientTO::from).collect(),
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&detail).unwrap()))
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
pub async fn retry_job<S: MailRestState>(
    state: State<S>,
    Path(id): Path<String>,
) -> Response {
    tracing::info!("retry_job called for job_id={}", id);
    error_handler(
        (async {
            let job_id = uuid::Uuid::parse_str(&id)
                .map_err(|_| MailServiceError::NotFound)?;
            let job = state.mail_service().retry_job(job_id).await?;
            let to = MailJobTO::from(&job);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to).unwrap()))
                .unwrap())
        })
        .await,
    )
}
