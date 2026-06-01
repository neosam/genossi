use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};

use crate::dao::MailTemplate;
use crate::mail_template_service::{MailTemplateError, MailTemplateService};

pub trait MailTemplateRestState: Clone + Send + Sync + 'static {
    type MailTemplateService: MailTemplateService;
    fn mail_template_service(&self) -> Arc<Self::MailTemplateService>;
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MailTemplateTO {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub body: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct CreateMailTemplateRequest {
    pub name: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateMailTemplateRequest {
    pub name: String,
    pub subject: String,
    pub body: String,
    pub version: String,
}

impl From<&MailTemplate> for MailTemplateTO {
    fn from(t: &MailTemplate) -> Self {
        Self {
            id: t.id.to_string(),
            name: t.name.to_string(),
            subject: t.subject.to_string(),
            body: t.body.to_string(),
            version: t.version.to_string(),
        }
    }
}

fn error_handler(result: Result<Response, MailTemplateError>) -> Response {
    match result {
        Ok(response) => response,
        Err(MailTemplateError::NotFound) => Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": "Not found"}).to_string(),
            ))
            .unwrap(),
        Err(MailTemplateError::DuplicateName(name)) => Response::builder()
            .status(409)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": format!("Template name '{}' already exists", name)})
                    .to_string(),
            ))
            .unwrap(),
        Err(MailTemplateError::VersionConflict) => Response::builder()
            .status(409)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": "Version conflict — template was modified by another user"})
                    .to_string(),
            ))
            .unwrap(),
        Err(MailTemplateError::BadRequest(msg)) => Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": msg.to_string()}).to_string(),
            ))
            .unwrap(),
        Err(MailTemplateError::DataAccess(msg)) => {
            tracing::error!("Mail template data access error: {}", msg);
            Response::builder()
                .status(500)
                .body(Body::from("Internal server error"))
                .unwrap()
        }
    }
}

pub fn generate_route<S: MailTemplateRestState>() -> Router<S> {
    Router::new()
        .route("/", get(list_templates::<S>).post(create_template::<S>))
        .route(
            "/{id}",
            get(get_template::<S>)
                .put(update_template::<S>)
                .delete(delete_template::<S>),
        )
}

#[derive(OpenApi)]
#[openapi(
    paths(list_templates, create_template, get_template, update_template, delete_template),
    components(schemas(MailTemplateTO, CreateMailTemplateRequest, UpdateMailTemplateRequest)),
    tags((name = "Mail Templates", description = "Email template management"))
)]
pub struct ApiDoc;

#[instrument(skip(state))]
#[utoipa::path(
    get,
    tag = "Mail Templates",
    path = "/",
    responses(
        (status = 200, description = "List of mail templates", body = Vec<MailTemplateTO>),
    ),
)]
async fn list_templates<S: MailTemplateRestState>(state: State<S>) -> Response {
    error_handler(
        (async {
            let templates = state.mail_template_service().list().await?;
            let tos: Vec<MailTemplateTO> = templates.iter().map(MailTemplateTO::from).collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&tos)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    post,
    tag = "Mail Templates",
    path = "/",
    request_body = CreateMailTemplateRequest,
    responses(
        (status = 201, description = "Template created", body = MailTemplateTO),
        (status = 409, description = "Duplicate name"),
    ),
)]
async fn create_template<S: MailTemplateRestState>(
    state: State<S>,
    axum::Json(body): axum::Json<CreateMailTemplateRequest>,
) -> Response {
    error_handler(
        (async {
            let tpl = state
                .mail_template_service()
                .create(&body.name, &body.subject, &body.body)
                .await?;
            let to = MailTemplateTO::from(&tpl);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    get,
    tag = "Mail Templates",
    path = "/{id}",
    params(("id" = String, Path, description = "Template UUID")),
    responses(
        (status = 200, description = "Template found", body = MailTemplateTO),
        (status = 404, description = "Template not found"),
    ),
)]
async fn get_template<S: MailTemplateRestState>(
    state: State<S>,
    Path(id): Path<String>,
) -> Response {
    error_handler(
        (async {
            let uuid = uuid::Uuid::parse_str(&id)
                .map_err(|_| MailTemplateError::BadRequest(Arc::from("Invalid UUID")))?;
            let tpl = state.mail_template_service().get(uuid).await?;
            let to = MailTemplateTO::from(&tpl);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    put,
    tag = "Mail Templates",
    path = "/{id}",
    params(("id" = String, Path, description = "Template UUID")),
    request_body = UpdateMailTemplateRequest,
    responses(
        (status = 200, description = "Template updated", body = MailTemplateTO),
        (status = 404, description = "Template not found"),
        (status = 409, description = "Version conflict or duplicate name"),
    ),
)]
async fn update_template<S: MailTemplateRestState>(
    state: State<S>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<UpdateMailTemplateRequest>,
) -> Response {
    error_handler(
        (async {
            let uuid = uuid::Uuid::parse_str(&id)
                .map_err(|_| MailTemplateError::BadRequest(Arc::from("Invalid UUID")))?;
            let version = uuid::Uuid::parse_str(&body.version)
                .map_err(|_| MailTemplateError::BadRequest(Arc::from("Invalid version UUID")))?;
            let tpl = state
                .mail_template_service()
                .update(uuid, &body.name, &body.subject, &body.body, version)
                .await?;
            let to = MailTemplateTO::from(&tpl);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(state))]
#[utoipa::path(
    delete,
    tag = "Mail Templates",
    path = "/{id}",
    params(("id" = String, Path, description = "Template UUID")),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 404, description = "Template not found"),
    ),
)]
async fn delete_template<S: MailTemplateRestState>(
    state: State<S>,
    Path(id): Path<String>,
) -> Response {
    error_handler(
        (async {
            let uuid = uuid::Uuid::parse_str(&id)
                .map_err(|_| MailTemplateError::BadRequest(Arc::from("Invalid UUID")))?;
            state.mail_template_service().delete(uuid).await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}
