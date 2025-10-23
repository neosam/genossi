
use axum::extract::Request;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;
#[cfg(feature = "oidc")]
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
#[cfg(feature = "oidc")]
use inventurly_service::session::SessionService;
#[cfg(feature = "oidc")]
use tower_cookies::Cookies;

#[cfg(feature = "oidc")]
use crate::Context;
use crate::RestStateDef;

#[cfg(feature = "oidc")]
pub async fn register_session<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    claims: Option<OidcClaims<EmptyAdditionalClaims>>,
    request: Request,
    next: Next,
) -> Response {
    use time::OffsetDateTime;
    use tower_cookies::Cookie;

    let cookies = request
        .extensions()
        .get::<Cookies>()
        .expect("Cookies extension not set");

    if let Some(oidc_claims) = claims {
        let username = oidc_claims
            .preferred_username()
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| "NoUsername".to_string());

        // Use the new method that ensures user exists before creating session
        let session = rest_state
            .session_service()
            .ensure_user_and_create_session(&username, 365 * 24 * 60 * 60) // 365 days in seconds
            .await
            .expect("Failed to create session for OIDC user");
        let session_id = session.session_id.to_string();
        let now = OffsetDateTime::now_utc();
        let expires = now + time::Duration::days(365);
        let cookie = Cookie::build(Cookie::new("app_session", session_id))
            .path("/")
            .expires(expires)
            .http_only(true)
            .same_site(tower_cookies::cookie::SameSite::Strict)
            .secure(true);
        cookies.add(cookie.into());
    }
    next.run(request).await
}

#[cfg(feature = "oidc")]
pub async fn context_extractor<RestState: RestStateDef>(
    State(rest_state): State<RestState>,
    mut request: Request,
    next: Next,
) -> Response {
    let cookies = request
        .extensions()
        .get::<Cookies>()
        .expect("Cookies extension not set");
    tracing::info!("All cookies: {:?}", cookies.list());

    tracing::info!("Search for app_session cookie");
    let auth_context = if let Some(cookie) = cookies.get("app_session") {
        tracing::info!("app_session cookie found: {:?}", cookie);
        let session_id = cookie.value();
        tracing::info!("Session ID: {:?}", session_id);
        if let Some(session) = rest_state
            .session_service()
            .verify_user_session(session_id)
            .await
            .unwrap()
        {
            tracing::info!("Session found: {:?}", session);
            // Extract auth context from session
            rest_state
                .session_service()
                .extract_auth_context(Some(session_id.to_string()))
                .await
                .ok()
                .flatten()
        } else {
            tracing::info!("Session not found");
            None
        }
    } else {
        tracing::info!("app_session cookie not found");
        None
    };

    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    let context = Context {
        auth: inventurly_service::permission::Authentication::Context(
            inventurly_service::permission::MockContext,
        ),
        auth_context,
    };
    
    #[cfg(feature = "oidc")]
    let context = {
        let auth = if let Some(ref auth_ctx) = auth_context {
            match auth_ctx {
                inventurly_service::auth_types::AuthContext::Mock(mock_ctx) => {
                    inventurly_service::permission::Authentication::Context(
                        inventurly_service::auth_types::AuthenticatedContext {
                            user_id: mock_ctx.user_id.clone(),
                        },
                    )
                }
                inventurly_service::auth_types::AuthContext::Oidc(user_id) => {
                    inventurly_service::permission::Authentication::Context(
                        inventurly_service::auth_types::AuthenticatedContext {
                            user_id: user_id.clone(),
                        },
                    )
                }
            }
        } else {
            inventurly_service::permission::Authentication::Full
        };
        
        Context {
            auth,
            auth_context,
        }
    };

    request.extensions_mut().insert(context);
    next.run(request).await
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub async fn context_extractor<RestState: RestStateDef>(
    State(_rest_state): State<RestState>,
    mut request: Request,
    next: Next,
) -> Response {
    let context = crate::auth_middleware::mock_auth_context();
    request.extensions_mut().insert(context);
    next.run(request).await
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub async fn forbid_unauthenticated<RestState: RestStateDef>(
    State(_rest_state): State<RestState>,
    request: Request,
    next: Next,
) -> Response {
    // In mock auth mode, always allow access
    next.run(request).await
}

#[cfg(feature = "oidc")]
pub async fn forbid_unauthenticated<RestState: RestStateDef>(
    State(_rest_state): State<RestState>,
    request: Request,
    next: Next,
) -> Response {
    use tracing::{info, warn};

    info!("Checking authentication");

    // Check if context exists and has auth_context
    let is_authenticated = request
        .extensions()
        .get::<Context>()
        .and_then(|ctx| ctx.auth_context.as_ref())
        .is_some();

    // Allow access to authenticate endpoint and swagger
    let is_public_path = request.uri().path().ends_with("/authenticate")
        || request.uri().path().starts_with("/swagger-ui");

    if is_authenticated || is_public_path {
        info!("Authenticated or public path");
        next.run(request).await
    } else {
        warn!("Not authenticated");
        Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap()
    }
}
