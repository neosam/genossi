use axum::{body::Body, extract::State, response::Response, routing::get, Extension, Router};
use genossi_config::service::ConfigService as _;
use genossi_service::user_preference::UserPreferenceService as _;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};

use crate::{Context, RestStateDef};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FooterResponse {
    pub footer: String,
}

pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new().route("/", get(get_footer::<RestState>))
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Mail Footer",
    path = "/",
    responses(
        (status = 200, description = "Rendered footer for current user", body = FooterResponse),
        (status = 400, description = "Invalid footer template"),
    ),
)]
pub async fn get_footer<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    let template = match rest_state.config_service().get("mail_footer").await {
        Ok(entry) => entry.value.to_string(),
        Err(_) => String::new(),
    };

    if template.is_empty() {
        let response = FooterResponse {
            footer: String::new(),
        };
        return Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&response).unwrap()))
            .unwrap();
    }

    let auth = match crate::extract_auth_context(Some(context)) {
        Ok(auth) => auth,
        Err(_) => {
            return Response::builder()
                .status(401)
                .body(Body::from("Unauthorized"))
                .unwrap();
        }
    };

    let sender_name = match rest_state
        .user_preference_service()
        .get_by_key("sender_name", auth, None)
        .await
    {
        Ok(pref) => pref.value.to_string(),
        Err(_) => String::new(),
    };

    match genossi_mail::template::render_footer(&template, &sender_name) {
        Ok(footer) => {
            let response = FooterResponse { footer };
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response).unwrap()))
                .unwrap()
        }
        Err(e) => Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({"error": e.message}).to_string(),
            ))
            .unwrap(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(get_footer),
    components(schemas(FooterResponse)),
    tags((name = "Mail Footer", description = "Mail footer rendering endpoint"))
)]
pub struct ApiDoc;
