use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_dao::application::ApplicationStatus;
use genossi_rest_types::{
    AdminCreateApplicationRequest, ApplicationStatusTO, ApplicationTO, PublicJoinRequest,
    PublicJoinResponse, UpdateApplicationRequest, ValidationErrorResponse, ValidationFailureItem,
};
use genossi_service::application::{ApplicationService, ApplicationSubmission, ApplicationUpdate};
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait ApplicationRestState: Clone + Send + Sync + 'static {
    type ApplicationService: ApplicationService<Context = crate::ContextType>
        + Send
        + Sync
        + 'static;

    fn application_service(&self) -> Arc<Self::ApplicationService>;
    fn get_config_value(
        &self,
        key: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>>;
}

#[derive(Debug, serde::Deserialize)]
pub struct ApplicationListQuery {
    pub status: Option<String>,
}

// --- Input validation for public join ---

fn validate_required_field(
    errors: &mut Vec<ValidationFailureItem>,
    field: &str,
    value: &str,
    max_len: usize,
) {
    if value.is_empty() {
        errors.push(ValidationFailureItem {
            field: field.to_string(),
            message: "missing".to_string(),
        });
    } else if value.len() > max_len {
        errors.push(ValidationFailureItem {
            field: field.to_string(),
            message: format!("too long (max {})", max_len),
        });
    }
}

pub fn validate_join_request(body: &PublicJoinRequest) -> Result<(), Vec<ValidationFailureItem>> {
    let mut errors = Vec::new();

    validate_required_field(&mut errors, "first_name", &body.first_name, 128);
    validate_required_field(&mut errors, "last_name", &body.last_name, 128);

    // Email: required, must contain '@', 3..=320
    if body.email.is_empty() {
        errors.push(ValidationFailureItem {
            field: "email".to_string(),
            message: "missing".to_string(),
        });
    } else if body.email.len() > 320 {
        errors.push(ValidationFailureItem {
            field: "email".to_string(),
            message: "too long (max 320)".to_string(),
        });
    } else if !body.email.contains('@') || body.email.len() < 3 {
        errors.push(ValidationFailureItem {
            field: "email".to_string(),
            message: "invalid email format".to_string(),
        });
    }

    validate_required_field(&mut errors, "street", &body.street, 128);
    validate_required_field(&mut errors, "house_number", &body.house_number, 32);
    validate_required_field(&mut errors, "postal_code", &body.postal_code, 16);
    validate_required_field(&mut errors, "city", &body.city, 128);

    // Title: optional, max 64
    if let Some(ref title) = body.title {
        if title.len() > 64 {
            errors.push(ValidationFailureItem {
                field: "title".to_string(),
                message: "too long (max 64)".to_string(),
            });
        }
    }

    // Shares: >= 1
    if body.shares < 1 {
        errors.push(ValidationFailureItem {
            field: "shares".to_string(),
            message: "shares must be >= 1".to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// --- Public endpoint (no auth, API key required) ---

#[instrument(skip(state, headers))]
#[utoipa::path(
    post,
    tag = "Public Join",
    path = "/api/public/join",
    request_body = PublicJoinRequest,
    params(
        ("X-Api-Key" = String, Header, description = "API key for public endpoint authentication"),
    ),
    responses(
        (status = 201, description = "Application submitted", body = PublicJoinResponse),
        (status = 401, description = "Invalid or missing API key"),
        (status = 422, description = "Validation error", body = ValidationErrorResponse),
        (status = 429, description = "Rate limit exceeded"),
    ),
)]
pub async fn public_join<S: ApplicationRestState>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(body): Json<PublicJoinRequest>,
) -> Response {
    error_handler(
        (async {
            // Validate API key (constant-time comparison to prevent timing side-channel)
            let api_key = headers
                .get("X-Api-Key")
                .and_then(|v| v.to_str().ok())
                .ok_or(RestError::Unauthorized)?;

            let stored_key = state
                .get_config_value("public_api_key")
                .await
                .ok_or(RestError::Unauthorized)?;

            if !constant_time_eq::constant_time_eq(api_key.as_bytes(), stored_key.as_bytes()) {
                return Err(RestError::Unauthorized);
            }

            // Validate input fields
            if let Err(validation_errors) = validate_join_request(&body) {
                let error_response = ValidationErrorResponse {
                    errors: validation_errors,
                };
                return Ok(Response::builder()
                    .status(422)
                    .header("Content-Type", "application/json")
                    .body(Body::new(serde_json::to_string(&error_response)?))
                    .unwrap());
            }

            let salutation = body
                .salutation
                .as_ref()
                .map(genossi_dao::member::Salutation::from);

            let submission = ApplicationSubmission {
                first_name: Arc::from(body.first_name.as_str()),
                last_name: Arc::from(body.last_name.as_str()),
                salutation,
                title: body.title.as_deref().map(Arc::from),
                email: Some(Arc::from(body.email.as_str())),
                street: Some(Arc::from(body.street.as_str())),
                house_number: Some(Arc::from(body.house_number.as_str())),
                postal_code: Some(Arc::from(body.postal_code.as_str())),
                city: Some(Arc::from(body.city.as_str())),
                shares: body.shares,
            };

            state
                .application_service()
                .submit(&submission, true)
                .await?;

            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&PublicJoinResponse {
                    message: "Beitrittserklärung eingegangen".to_string(),
                })?))
                .unwrap())
        })
        .await,
    )
}

pub fn generate_public_route<S: ApplicationRestState>() -> Router<S> {
    Router::new().route("/join", post(public_join::<S>))
}

// --- Admin endpoints (auth required) ---

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Applications",
    path = "",
    params(
        ("status" = Option<String>, Query, description = "Filter by status (Offen, Bestaetigt, Abgelehnt)")
    ),
    responses(
        (status = 200, description = "List applications", body = [ApplicationTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_applications<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<ApplicationListQuery>,
) -> Response {
    error_handler(
        (async {
            let status_filter = query
                .status
                .as_deref()
                .map(|s| match s {
                    "Offen" => Ok(ApplicationStatus::Offen),
                    "Bestaetigt" => Ok(ApplicationStatus::Bestaetigt),
                    "Abgelehnt" => Ok(ApplicationStatus::Abgelehnt),
                    other => Err(RestError::BadRequest(format!("Unknown status: {}", other))),
                })
                .transpose()?;

            let apps: Arc<[ApplicationTO]> = rest_state
                .application_service()
                .list(status_filter, crate::extract_auth_context(Some(context))?)
                .await?
                .iter()
                .map(ApplicationTO::from)
                .collect();

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&apps)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Applications",
    path = "",
    request_body = AdminCreateApplicationRequest,
    responses(
        (status = 201, description = "Application created", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation error"),
    ),
)]
pub async fn create_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<AdminCreateApplicationRequest>,
) -> Response {
    error_handler(
        (async {
            crate::extract_auth_context(Some(context))?;

            let salutation = body
                .salutation
                .as_ref()
                .map(genossi_dao::member::Salutation::from);
            let send_mail = body.send_mail.unwrap_or(false);

            let submission = ApplicationSubmission {
                first_name: Arc::from(body.first_name.as_str()),
                last_name: Arc::from(body.last_name.as_str()),
                salutation,
                title: body.title.as_deref().map(Arc::from),
                email: body.email.as_deref().map(Arc::from),
                street: body.street.as_deref().map(Arc::from),
                house_number: body.house_number.as_deref().map(Arc::from),
                postal_code: body.postal_code.as_deref().map(Arc::from),
                city: body.city.as_deref().map(Arc::from),
                shares: body.shares,
            };

            let app = rest_state
                .application_service()
                .submit(&submission, send_mail)
                .await?;

            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(
                    &app,
                ))?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Applications",
    path = "/{id}",
    responses(
        (status = 200, description = "Get application", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
)]
pub async fn get_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let app = rest_state
                .application_service()
                .get(id, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(
                    &app,
                ))?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Applications",
    path = "/{id}/confirm",
    responses(
        (status = 200, description = "Application confirmed, member created", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict - application not in Offen status"),
    ),
)]
pub async fn confirm_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let app = rest_state
                .application_service()
                .confirm(id, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(
                    &app,
                ))?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Applications",
    path = "/{id}/reject",
    responses(
        (status = 200, description = "Application rejected", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict - application not in Offen status"),
    ),
)]
pub async fn reject_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let app = rest_state
                .application_service()
                .reject(id, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(
                    &app,
                ))?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Applications",
    path = "/{id}",
    request_body = UpdateApplicationRequest,
    responses(
        (status = 200, description = "Application updated", body = ApplicationTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Version conflict"),
        (status = 422, description = "Validation error"),
    ),
)]
pub async fn update_application<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateApplicationRequest>,
) -> Response {
    error_handler(
        (async {
            let salutation = body
                .salutation
                .as_ref()
                .map(genossi_dao::member::Salutation::from);

            let update = ApplicationUpdate {
                first_name: Arc::from(body.first_name.as_str()),
                last_name: Arc::from(body.last_name.as_str()),
                salutation,
                title: body.title.as_deref().map(Arc::from),
                email: body.email.as_deref().map(Arc::from),
                street: body.street.as_deref().map(Arc::from),
                house_number: body.house_number.as_deref().map(Arc::from),
                postal_code: body.postal_code.as_deref().map(Arc::from),
                city: body.city.as_deref().map(Arc::from),
                shares: body.shares,
                version: body.version,
            };

            let app = rest_state
                .application_service()
                .update_application(id, &update, crate::extract_auth_context(Some(context))?)
                .await?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&ApplicationTO::from(
                    &app,
                ))?))
                .unwrap())
        })
        .await,
    )
}

pub fn generate_route<RestState: RestStateDef + ApplicationRestState>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            get(list_applications::<RestState>).post(create_application::<RestState>),
        )
        .route(
            "/{id}",
            get(get_application::<RestState>).put(update_application::<RestState>),
        )
        .route("/{id}/confirm", post(confirm_application::<RestState>))
        .route("/{id}/reject", post(reject_application::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_applications,
        create_application,
        get_application,
        update_application,
        confirm_application,
        reject_application
    ),
    components(schemas(
        ApplicationTO,
        ApplicationStatusTO,
        AdminCreateApplicationRequest,
        UpdateApplicationRequest,
        PublicJoinResponse
    ))
)]
pub struct ApiDoc;

#[derive(OpenApi)]
#[openapi(
    paths(public_join),
    components(schemas(
        PublicJoinRequest,
        PublicJoinResponse,
        ValidationErrorResponse,
        ValidationFailureItem
    ))
)]
pub struct PublicApiDoc;

#[cfg(test)]
mod tests {
    use super::*;
    use genossi_rest_types::SalutationTO;

    fn valid_request() -> PublicJoinRequest {
        PublicJoinRequest {
            first_name: "Max".to_string(),
            last_name: "Mustermann".to_string(),
            salutation: Some(SalutationTO::Herr),
            title: None,
            email: "max@example.com".to_string(),
            street: "Musterstraße".to_string(),
            house_number: "42".to_string(),
            postal_code: "12345".to_string(),
            city: "Berlin".to_string(),
            shares: 2,
        }
    }

    #[test]
    fn test_validate_valid_request() {
        assert!(validate_join_request(&valid_request()).is_ok());
    }

    #[test]
    fn test_validate_empty_first_name() {
        let mut req = valid_request();
        req.first_name = "".to_string();
        let errors = validate_join_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "first_name" && e.message == "missing"));
    }

    #[test]
    fn test_validate_first_name_too_long() {
        let mut req = valid_request();
        req.first_name = "a".repeat(200);
        let errors = validate_join_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "first_name" && e.message.contains("too long")));
    }

    #[test]
    fn test_validate_email_invalid_format() {
        let mut req = valid_request();
        req.email = "foo".to_string();
        let errors = validate_join_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "email" && e.message == "invalid email format"));
    }

    #[test]
    fn test_validate_shares_zero() {
        let mut req = valid_request();
        req.shares = 0;
        let errors = validate_join_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "shares" && e.message == "shares must be >= 1"));
    }

    #[test]
    fn test_validate_multiple_errors() {
        let mut req = valid_request();
        req.email = "".to_string();
        req.shares = 0;
        let errors = validate_join_request(&req).unwrap_err();
        assert!(errors.iter().any(|e| e.field == "email"));
        assert!(errors.iter().any(|e| e.field == "shares"));
    }

    #[test]
    fn test_validate_optional_title_too_long() {
        let mut req = valid_request();
        req.title = Some("a".repeat(100));
        let errors = validate_join_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "title" && e.message.contains("too long")));
    }

    #[test]
    fn test_validate_valid_without_title() {
        let mut req = valid_request();
        req.title = None;
        assert!(validate_join_request(&req).is_ok());
    }
}
