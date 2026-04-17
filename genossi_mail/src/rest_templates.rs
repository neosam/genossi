use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post, put};
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

fn format_datetime(dt: &time::PrimitiveDateTime) -> String {
    dt.assume_utc()
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| dt.to_string())
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

fn error_response(err: MailTemplateError) -> Response {
    match err {
        MailTemplateError::NotFound => Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": "Not found"}).to_string(),
            ))
            .unwrap(),
        MailTemplateError::DuplicateName(name) => Response::builder()
            .status(409)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": format!("Template name '{}' already exists", name)})
                    .to_string(),
            ))
            .unwrap(),
        MailTemplateError::VersionConflict => Response::builder()
            .status(409)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": "Version conflict — template was modified by another user"})
                    .to_string(),
            ))
            .unwrap(),
        MailTemplateError::DataAccess(msg) => {
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
    match state.mail_template_service().list().await {
        Ok(templates) => {
            let tos: Vec<MailTemplateTO> = templates.iter().map(MailTemplateTO::from).collect();
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&tos).unwrap()))
                .unwrap()
        }
        Err(e) => error_response(e),
    }
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
    match state
        .mail_template_service()
        .create(&body.name, &body.subject, &body.body)
        .await
    {
        Ok(tpl) => {
            let to = MailTemplateTO::from(&tpl);
            Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&to).unwrap()))
                .unwrap()
        }
        Err(e) => error_response(e),
    }
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
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"error": "Invalid UUID"}).to_string(),
                ))
                .unwrap()
        }
    };

    match state.mail_template_service().get(uuid).await {
        Ok(tpl) => {
            let to = MailTemplateTO::from(&tpl);
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&to).unwrap()))
                .unwrap()
        }
        Err(e) => error_response(e),
    }
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
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"error": "Invalid UUID"}).to_string(),
                ))
                .unwrap()
        }
    };

    let version = match uuid::Uuid::parse_str(&body.version) {
        Ok(v) => v,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"error": "Invalid version UUID"}).to_string(),
                ))
                .unwrap()
        }
    };

    match state
        .mail_template_service()
        .update(uuid, &body.name, &body.subject, &body.body, version)
        .await
    {
        Ok(tpl) => {
            let to = MailTemplateTO::from(&tpl);
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&to).unwrap()))
                .unwrap()
        }
        Err(e) => error_response(e),
    }
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
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"error": "Invalid UUID"}).to_string(),
                ))
                .unwrap()
        }
    };

    match state.mail_template_service().delete(uuid).await {
        Ok(()) => Response::builder().status(204).body(Body::empty()).unwrap(),
        Err(e) => error_response(e),
    }
}
