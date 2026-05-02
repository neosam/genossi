use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_rest_types::{
    AssemblyDetailTO, AssemblyStatusTO, AssemblyTO, CreateAssemblyRequest, UpdateAssemblyRequest,
    ValidationFailureItem,
};
use genossi_service::assembly::{AssemblyService, AssemblySubmission, AssemblyUpdate};
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait AssemblyRestState: Clone + Send + Sync + 'static {
    type AssemblyService: AssemblyService<Context = crate::ContextType> + Send + Sync + 'static;

    fn assembly_service(&self) -> Arc<Self::AssemblyService>;
}

// --- Validation helpers (mirror application.rs::validate_required_field) ---

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

fn validate_optional_max_len(
    errors: &mut Vec<ValidationFailureItem>,
    field: &str,
    value: &Option<String>,
    max_len: usize,
) {
    if let Some(v) = value {
        if v.len() > max_len {
            errors.push(ValidationFailureItem {
                field: field.to_string(),
                message: format!("too long (max {})", max_len),
            });
        }
    }
}

pub fn validate_create_assembly_request(
    body: &CreateAssemblyRequest,
) -> Result<(), Vec<ValidationFailureItem>> {
    let mut errors = Vec::new();
    validate_required_field(&mut errors, "name", &body.name, 256);
    validate_optional_max_len(&mut errors, "location", &body.location, 256);
    if body.date.is_none() {
        errors.push(ValidationFailureItem {
            field: "date".to_string(),
            message: "missing".to_string(),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_update_assembly_request(
    body: &UpdateAssemblyRequest,
) -> Result<(), Vec<ValidationFailureItem>> {
    let mut errors = Vec::new();
    validate_required_field(&mut errors, "name", &body.name, 256);
    validate_optional_max_len(&mut errors, "location", &body.location, 256);
    if body.date.is_none() {
        errors.push(ValidationFailureItem {
            field: "date".to_string(),
            message: "missing".to_string(),
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// --- Handlers ---

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Assemblies",
    path = "",
    responses(
        (status = 200, description = "List assemblies", body = [AssemblyTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_assemblies<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let assemblies = rest_state
                .assembly_service()
                .get_all_assemblies(auth)
                .await?;
            let to_list: Vec<AssemblyTO> = assemblies.iter().map(AssemblyTO::from).collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to_list)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Assemblies",
    path = "",
    request_body = CreateAssemblyRequest,
    responses(
        (status = 201, description = "Created", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation Error"),
    ),
)]
pub async fn create_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<CreateAssemblyRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            validate_create_assembly_request(&body).map_err(|errs| {
                let messages: Vec<String> = errs
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect();
                RestError::BadRequest(format!("Validation failed: {}", messages.join(", ")))
            })?;
            let date = body
                .date
                .ok_or_else(|| RestError::BadRequest("date required".into()))?;
            let submission = AssemblySubmission {
                name: Arc::from(body.name.as_str()),
                date,
                location: body.location.as_deref().map(Arc::from),
            };
            let assembly = rest_state
                .assembly_service()
                .create_assembly(&submission, auth)
                .await?;
            let to = AssemblyTO::from(&assembly);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Assemblies",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "Assembly detail", body = AssemblyDetailTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
)]
pub async fn get_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let detail = rest_state.assembly_service().get_assembly(id, auth).await?;
            let to = AssemblyDetailTO::from(&detail);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    put,
    tag = "Assemblies",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "Assembly ID")),
    request_body = UpdateAssemblyRequest,
    responses(
        (status = 200, description = "Updated", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation, or version mismatch)"),
        (status = 422, description = "Validation Error"),
    ),
)]
pub async fn update_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateAssemblyRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            validate_update_assembly_request(&body).map_err(|errs| {
                let messages: Vec<String> = errs
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect();
                RestError::BadRequest(format!("Validation failed: {}", messages.join(", ")))
            })?;
            let date = body
                .date
                .ok_or_else(|| RestError::BadRequest("date required".into()))?;
            let update = AssemblyUpdate {
                name: Arc::from(body.name.as_str()),
                date,
                location: body.location.as_deref().map(Arc::from),
                version: body.version,
            };
            let assembly = rest_state
                .assembly_service()
                .update_assembly(id, &update, auth)
                .await?;
            let to = AssemblyTO::from(&assembly);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Assemblies",
    path = "/{id}/open",
    params(("id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "Opened", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation)"),
    ),
)]
pub async fn open_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let assembly = rest_state
                .assembly_service()
                .open_assembly(id, auth)
                .await?;
            let to = AssemblyTO::from(&assembly);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Assemblies",
    path = "/{id}/close",
    params(("id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "Closed", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Open)"),
    ),
)]
pub async fn close_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let assembly = rest_state
                .assembly_service()
                .close_assembly(id, auth)
                .await?;
            let to = AssemblyTO::from(&assembly);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

pub fn generate_route<RestState: RestStateDef + AssemblyRestState>() -> Router<RestState> {
    Router::new()
        .route(
            "/",
            get(list_assemblies::<RestState>).post(create_assembly::<RestState>),
        )
        .route(
            "/{id}",
            get(get_assembly::<RestState>).put(update_assembly::<RestState>),
        )
        .route("/{id}/open", post(open_assembly::<RestState>))
        .route("/{id}/close", post(close_assembly::<RestState>))
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_assemblies,
        create_assembly,
        get_assembly,
        update_assembly,
        open_assembly,
        close_assembly
    ),
    components(schemas(
        AssemblyTO,
        AssemblyStatusTO,
        AssemblyDetailTO,
        CreateAssemblyRequest,
        UpdateAssemblyRequest
    ))
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month, PrimitiveDateTime, Time};
    use uuid::Uuid;

    fn sample_datetime() -> PrimitiveDateTime {
        let date = Date::from_calendar_date(2026, Month::May, 15).unwrap();
        PrimitiveDateTime::new(date, Time::MIDNIGHT)
    }

    fn valid_create_request() -> CreateAssemblyRequest {
        CreateAssemblyRequest {
            name: "GV 2026".to_string(),
            date: Some(sample_datetime()),
            location: Some("Vereinsheim".to_string()),
        }
    }

    fn valid_update_request() -> UpdateAssemblyRequest {
        UpdateAssemblyRequest {
            name: "GV 2026 (renamed)".to_string(),
            date: Some(sample_datetime()),
            location: Some("Vereinsheim".to_string()),
            version: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_validate_create_assembly_request_valid() {
        assert!(validate_create_assembly_request(&valid_create_request()).is_ok());
    }

    #[test]
    fn test_validate_create_assembly_request_empty_name() {
        let mut req = valid_create_request();
        req.name = "".to_string();
        let errors = validate_create_assembly_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "name" && e.message == "missing"));
    }

    #[test]
    fn test_validate_create_assembly_request_long_name() {
        let mut req = valid_create_request();
        req.name = "a".repeat(257);
        let errors = validate_create_assembly_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "name" && e.message.contains("too long")));
    }

    #[test]
    fn test_validate_create_assembly_request_long_location() {
        let mut req = valid_create_request();
        req.location = Some("a".repeat(257));
        let errors = validate_create_assembly_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "location" && e.message.contains("too long")));
    }

    #[test]
    fn test_validate_create_assembly_request_missing_date() {
        let mut req = valid_create_request();
        req.date = None;
        let errors = validate_create_assembly_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "date" && e.message == "missing"));
    }

    #[test]
    fn test_validate_create_assembly_request_optional_location_none_ok() {
        let mut req = valid_create_request();
        req.location = None;
        assert!(validate_create_assembly_request(&req).is_ok());
    }

    #[test]
    fn test_validate_update_assembly_request_valid() {
        let req = valid_update_request();
        assert!(validate_update_assembly_request(&req).is_ok());
        // version is mandatory by type — verify the fixture supplies a real (non-nil) UUID.
        assert_ne!(
            req.version,
            Uuid::nil(),
            "version must be a real UUID, not nil"
        );
    }

    #[test]
    fn test_validate_update_assembly_request_empty_name() {
        let mut req = valid_update_request();
        req.name = "".to_string();
        let errors = validate_update_assembly_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "name" && e.message == "missing"));
    }

    #[test]
    fn test_validate_update_assembly_request_missing_date() {
        let mut req = valid_update_request();
        req.date = None;
        let errors = validate_update_assembly_request(&req).unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "date" && e.message == "missing"));
    }
}
