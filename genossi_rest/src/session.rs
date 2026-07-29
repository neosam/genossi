use axum::extract::Request;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;
#[cfg(feature = "oidc")]
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use genossi_service::permission::MockContext;
#[cfg(feature = "oidc")]
use genossi_service::session::SessionService;
#[cfg(feature = "oidc")]
use tower_cookies::Cookies;

#[cfg(feature = "oidc")]
use crate::Context;
use crate::RestStateDef;

/// Normalizes a raw OIDC claim value into the `user_id` used throughout the
/// permission system.
///
/// Returns `None` when the claim carries no usable identity. Callers must
/// refuse the login in that case: falling back to a placeholder name would
/// funnel every user into one auto-registered account without roles or
/// privileges — a silent failure that looks like a broken login while
/// actually being a shared identity.
///
/// Compiled for `test` as well as `oidc` so the default (`mock_auth`) test
/// build still covers it, without leaving dead code in a non-OIDC release.
#[cfg(any(feature = "oidc", test))]
fn normalize_username(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

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
        // `sub` is the identity source: OIDC Core 1.0 requires it in every ID
        // token, so `openidconnect` hands it out unconditionally rather than as
        // an `Option`. The previously used `preferred_username` is optional and
        // stopped being issued by our provider.
        let Some(username) = normalize_username(oidc_claims.subject().as_str()) else {
            tracing::error!("OIDC id token carried an empty sub claim - refusing login");
            return Response::builder()
                .status(500)
                .body("Internal Server Error".into())
                .unwrap();
        };

        const SESSION_ABSOLUTE_LIFETIME_SECS: i64 = 365 * 24 * 60 * 60;

        match rest_state
            .session_service()
            .ensure_user_and_create_session(&username, SESSION_ABSOLUTE_LIFETIME_SECS)
            .await
        {
            Ok(session) => {
                let session_id = session.session_id.to_string();
                let now = OffsetDateTime::now_utc();
                let expires = now + time::Duration::seconds(SESSION_ABSOLUTE_LIFETIME_SECS);
                let cookie = Cookie::build(Cookie::new("app_session", session_id))
                    .path("/")
                    .expires(expires)
                    .http_only(true)
                    .same_site(tower_cookies::cookie::SameSite::Strict)
                    .secure(true);
                cookies.add(cookie.into());
            }
            Err(e) => {
                tracing::error!(error = %e, user_id = %username, "failed to create session");
                return Response::builder()
                    .status(500)
                    .body("Internal Server Error".into())
                    .unwrap();
            }
        }
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

    if let Some(cookie) = cookies.get("app_session") {
        let session_id = cookie.value();
        match rest_state
            .session_service()
            .verify_user_session(session_id)
            .await
        {
            Ok(Some(session)) => {
                tracing::debug!(user_id = %session.user_id, "session verified");
                let auth_context = genossi_service::auth_types::AuthenticatedContext {
                    user_id: session.user_id,
                    claims: session.claims,
                };
                request.extensions_mut().insert(Some(auth_context));
            }
            Ok(None) => {
                tracing::debug!("session invalid or expired");
                request
                    .extensions_mut()
                    .insert(None::<genossi_service::auth_types::AuthenticatedContext>);
            }
            Err(e) => {
                tracing::error!(error = %e, "session verification failed");
                request
                    .extensions_mut()
                    .insert(None::<genossi_service::auth_types::AuthenticatedContext>);
            }
        }
    } else {
        tracing::debug!("no session cookie");
        request
            .extensions_mut()
            .insert(None::<genossi_service::auth_types::AuthenticatedContext>);
    };

    next.run(request).await
}

#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub async fn context_extractor<RestState: RestStateDef>(
    State(_rest_state): State<RestState>,
    mut request: Request,
    next: Next,
) -> Response {
    request.extensions_mut().insert(MockContext);
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

    // Check if context exists and has user ID (simplified like shifty)
    let is_authenticated = request.extensions().get::<Context>().is_some()
        && request.extensions().get::<Context>().unwrap().is_some();

    // Allow access to authenticate endpoint, token login, and swagger
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

#[cfg(test)]
mod tests {
    use super::normalize_username;

    #[test]
    fn keeps_a_plain_subject_unchanged() {
        assert_eq!(normalize_username("simon"), Some("simon".to_string()));
    }

    #[test]
    fn keeps_an_opaque_provider_subject_unchanged() {
        // Providers are free to issue opaque `sub` values; we must not mangle
        // them, since they are the key the users table is matched on.
        let opaque = "1f2c9a4e-7b18-4c3d-9e55-0a6b8d2f4711";
        assert_eq!(normalize_username(opaque), Some(opaque.to_string()));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_username("  simon\n"), Some("simon".to_string()));
    }

    #[test]
    fn rejects_an_empty_subject() {
        assert_eq!(normalize_username(""), None);
    }

    #[test]
    fn rejects_a_whitespace_only_subject() {
        assert_eq!(normalize_username("   \t "), None);
    }
}
