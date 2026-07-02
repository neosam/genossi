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
    /// Phase 23 (HTML-01, HTML-05): read-only exposure of the persisted
    /// (ammonia-sanitized) `MailTemplate.body_html`. Backward-compatible:
    /// pre-Phase-24 clients see the same wire shape when this is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo</p>")]
    pub body_html: Option<String>,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct CreateMailTemplateRequest {
    pub name: String,
    pub subject: String,
    pub body: String,
    /// Phase 23 (HTML-01, HTML-05, D-03 EP2): optional author HTML.
    /// Sanitized server-side (ammonia) before persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo</p>")]
    pub body_html: Option<String>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateMailTemplateRequest {
    pub name: String,
    pub subject: String,
    pub body: String,
    /// Phase 23 (HTML-01, HTML-05, D-03 EP3): optional author HTML.
    /// Sanitized server-side (ammonia) before persistence. `None` clears
    /// the prior HTML sibling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(example = "<p>Hallo</p>")]
    pub body_html: Option<String>,
    pub version: String,
}

impl From<&MailTemplate> for MailTemplateTO {
    fn from(t: &MailTemplate) -> Self {
        Self {
            id: t.id.to_string(),
            name: t.name.to_string(),
            subject: t.subject.to_string(),
            body: t.body.to_string(),
            // Phase 23 Plan 04 (HTML-01): expose the sanitized HTML sibling.
            body_html: t.body_html.as_deref().map(String::from),
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
                // Phase 23 Plan 04 (HTML-05, D-03 EP2): forward body.body_html
                // — service layer sanitizes before persist.
                .create(
                    &body.name,
                    &body.subject,
                    &body.body,
                    body.body_html.clone(),
                )
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
                // Phase 23 Plan 04 (HTML-05, D-03 EP3): forward body.body_html
                // — service layer sanitizes before persist. Update takes full
                // ownership of body_html; None clears the prior HTML.
                .update(
                    uuid,
                    &body.name,
                    &body.subject,
                    &body.body,
                    body.body_html.clone(),
                    version,
                )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Phase 23 Plan 04: MailTemplateTO with `body_html: None` MUST omit the
    /// key entirely (backward-compat contract for pre-Phase-24 clients).
    /// Locks the `skip_serializing_if = "Option::is_none"` attribute.
    #[test]
    fn mail_template_to_serializes_without_body_html_when_none() {
        let to = MailTemplateTO {
            id: "abc".to_string(),
            name: "T".to_string(),
            subject: "S".to_string(),
            body: "B".to_string(),
            body_html: None,
            version: "v1".to_string(),
        };
        let json = serde_json::to_string(&to).unwrap();
        assert!(
            !json.contains("body_html"),
            "None body_html must be omitted from wire (skip_serializing_if), got: {}",
            json
        );

        // Positive control: Some(...) DOES appear on the wire.
        let to_html = MailTemplateTO {
            body_html: Some("<p>hi</p>".to_string()),
            ..to
        };
        let json_html = serde_json::to_string(&to_html).unwrap();
        assert!(
            json_html.contains("body_html"),
            "Some body_html must appear on wire, got: {}",
            json_html
        );
    }
}
