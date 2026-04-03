use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, put};
use axum::{Extension, Router};
use genossi_service::template::TemplateError;

use genossi_service::member::MemberService;
use genossi_service::permission::PermissionService;

use crate::{error_handler, Context, RestError, RestStateDef};

fn template_error_to_rest(e: TemplateError) -> RestError {
    match e {
        TemplateError::NotFound => RestError::NotFound,
        TemplateError::PathTraversal => RestError::BadRequest("Invalid path".to_string()),
        TemplateError::DirectoryNotEmpty => {
            RestError::BadRequest("Directory is not empty".to_string())
        }
        TemplateError::IoError(msg) => RestError::InternalError(msg.to_string()),
        TemplateError::RenderError(msg) => RestError::BadRequest(msg.to_string()),
    }
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_templates::<RestState>))
        .route("/{*path}", get(read_template::<RestState>))
        .route("/{*path}", put(write_template::<RestState>))
        .route("/{*path}", delete(delete_template::<RestState>))
}

pub fn generate_render_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new().route("/{*path}", axum::routing::post(render_template::<RestState>))
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "File tree listing"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Templates"
)]
async fn list_templates<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            // Check permission
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .permission_service()
                .check_permission("manage_members", auth)
                .await
                .map_err(|_| RestError::Unauthorized)?;

            let tree = rest_state
                .template_storage()
                .list_tree()
                .await
                .map_err(template_error_to_rest)?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&tree).unwrap()))
                .unwrap())
        })
        .await,
    )
}

#[utoipa::path(
    get,
    path = "/{path}",
    params(
        ("path" = String, Path, description = "Template file path"),
    ),
    responses(
        (status = 200, description = "Template content"),
        (status = 404, description = "Template not found"),
    ),
    tag = "Templates"
)]
async fn read_template<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(path): Path<String>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .permission_service()
                .check_permission("manage_members", auth)
                .await
                .map_err(|_| RestError::Unauthorized)?;

            let content = rest_state
                .template_storage()
                .read_file(&path)
                .await
                .map_err(template_error_to_rest)?;

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(Body::new(content))
                .unwrap())
        })
        .await,
    )
}

#[utoipa::path(
    put,
    path = "/{path}",
    params(
        ("path" = String, Path, description = "Template file path"),
    ),
    responses(
        (status = 200, description = "Template created or updated"),
        (status = 400, description = "Invalid path"),
    ),
    tag = "Templates"
)]
async fn write_template<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(path): Path<String>,
    body: String,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .permission_service()
                .check_permission("manage_members", auth)
                .await
                .map_err(|_| RestError::Unauthorized)?;

            // Trailing slash means directory creation
            if path.ends_with('/') {
                rest_state
                    .template_storage()
                    .create_directory(&path.trim_end_matches('/'))
                    .await
                    .map_err(template_error_to_rest)?;
            } else {
                rest_state
                    .template_storage()
                    .write_file(&path, &body)
                    .await
                    .map_err(template_error_to_rest)?;
            }

            Ok(Response::builder()
                .status(200)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

#[utoipa::path(
    delete,
    path = "/{path}",
    params(
        ("path" = String, Path, description = "Template file path"),
    ),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 400, description = "Directory not empty"),
        (status = 404, description = "Template not found"),
    ),
    tag = "Templates"
)]
async fn delete_template<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(path): Path<String>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .permission_service()
                .check_permission("manage_members", auth)
                .await
                .map_err(|_| RestError::Unauthorized)?;

            rest_state
                .template_storage()
                .delete(&path)
                .await
                .map_err(template_error_to_rest)?;

            Ok(Response::builder()
                .status(204)
                .body(Body::empty())
                .unwrap())
        })
        .await,
    )
}

#[utoipa::path(
    post,
    path = "/{path}",
    params(
        ("path" = String, Path, description = "Template file path"),
    ),
    responses(
        (status = 200, description = "Rendered PDF"),
        (status = 400, description = "Render error"),
        (status = 404, description = "Template or member not found"),
    ),
    tag = "Templates"
)]
async fn render_template<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(path): Path<String>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            rest_state
                .permission_service()
                .check_permission("manage_members", auth.clone())
                .await
                .map_err(|_| RestError::Unauthorized)?;

            // Extract member_id from the path: the path is like "template.typ/render/{member_id}"
            // We need to split the render/{member_id} part from the template path
            let (template_path, member_id_str) = parse_render_path(&path)?;

            let member_id: uuid::Uuid = member_id_str
                .parse()
                .map_err(|_| RestError::BadRequest("Invalid member ID".to_string()))?;

            // Get member data
            let member = rest_state
                .member_service()
                .get(member_id, auth, None)
                .await
                .map_err(RestError::from)?;

            // Render PDF
            let pdf_bytes = rest_state
                .pdf_generator()
                .render(
                    &template_path,
                    rest_state.template_storage().base_path(),
                    &member,
                )
                .map_err(template_error_to_rest)?;

            // Derive filename from template path
            let filename = template_path
                .rsplit('/')
                .next()
                .unwrap_or(&template_path)
                .replace(".typ", ".pdf");

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/pdf")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(Body::from(pdf_bytes))
                .unwrap())
        })
        .await,
    )
}

/// Parse render path: "template/path.typ/{member_id}" -> ("template/path.typ", "{member_id}")
/// The path comes from /api/templates/render/*path where path is "template.typ/{member_id}"
fn parse_render_path(path: &str) -> Result<(String, String), RestError> {
    // The last segment is the member_id, everything before is the template path
    if let Some(pos) = path.rfind('/') {
        let template_path = &path[..pos];
        let member_id = &path[pos + 1..];
        if template_path.is_empty() || member_id.is_empty() {
            return Err(RestError::BadRequest(
                "Path must be template_path/member_id".to_string(),
            ));
        }
        Ok((template_path.to_string(), member_id.to_string()))
    } else {
        Err(RestError::BadRequest(
            "Path must be template_path/member_id".to_string(),
        ))
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        list_templates,
        read_template,
        write_template,
        delete_template,
        render_template,
    ),
    tags(
        (name = "Templates", description = "Template management and PDF generation")
    )
)]
pub struct ApiDoc;
