use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use inventurly_service::{auth_types::AuthContext, permission::PermissionService, ServiceError};
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
use inventurly_service::permission::MockContext;
#[cfg(feature = "oidc")]
use inventurly_service::auth_types::AuthenticatedContext;

use crate::RestStateDef;

/// Extract authentication context from request headers (session cookie or Bearer token)
pub async fn extract_auth_context<S: RestStateDef>(
    State(state): State<S>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_context =
        match extract_context_from_headers(&headers, state.session_service().as_ref()).await {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("Auth context extraction error: {:?}", err);
                None
            }
        };

    // Add the auth context to request extensions
    request.extensions_mut().insert(auth_context);

    next.run(request).await
}

/// Middleware that requires authentication - returns 401 if no valid auth context
pub async fn require_authentication<S: RestStateDef>(request: Request, next: Next) -> Response {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    let is_authenticated = request.extensions().get::<crate::Context>().is_some();
    
    #[cfg(feature = "oidc")]
    let is_authenticated = request.extensions().get::<crate::Context>()
        .map(|ctx| ctx.is_some())
        .unwrap_or(false);

    if is_authenticated {
        next.run(request).await
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized".into())
            .unwrap()
    }
}

/// Middleware that requires admin privileges - returns 403 if not admin
pub async fn require_admin<S: RestStateDef>(
    State(state): State<S>,
    request: Request,
    next: Next,
) -> Response {
    let context = request.extensions().get::<crate::Context>();

    if let Some(ctx) = context {
        // Use the helper function to extract auth
        match crate::extract_auth_context(Some(ctx.clone())) {
            Ok(auth) => {
                let permission_service = state.permission_service();
                match permission_service.check_permission("admin", auth).await {
                    Ok(()) => next.run(request).await,
                    Err(ServiceError::PermissionDenied) => Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body("Forbidden".into())
                        .unwrap(),
                    Err(ServiceError::Unauthorized) => Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body("Unauthorized".into())
                        .unwrap(),
                    Err(_) => Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body("Internal Server Error".into())
                        .unwrap(),
                }
            }
            Err(_) => Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Unauthorized".into())
                .unwrap(),
        }
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized".into())
            .unwrap()
    }
}

/// Extract authentication context from various header sources
async fn extract_context_from_headers<
    SessionService: inventurly_service::session::SessionService,
>(
    headers: &HeaderMap,
    session_service: &SessionService,
) -> Result<Option<AuthContext>, ServiceError> {
    // Try session cookie first
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(session_id) = extract_session_from_cookie(cookie_str) {
                if let Some(context) = session_service
                    .extract_auth_context(Some(session_id))
                    .await?
                {
                    return Ok(Some(context));
                }
            }
        }
    }

    // Try Authorization Bearer token (for API access)
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = extract_bearer_token(auth_str) {
                // Treat bearer token as session ID for now
                // In a real implementation, this might validate JWT tokens differently
                if let Some(context) = session_service.extract_auth_context(Some(token)).await? {
                    return Ok(Some(context));
                }
            }
        }
    }

    Ok(None)
}

/// Extract session ID from cookie string
fn extract_session_from_cookie(cookie_str: &str) -> Option<String> {
    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=') {
            if name.trim() == "app_session" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Extract bearer token from Authorization header
fn extract_bearer_token(auth_str: &str) -> Option<String> {
    if auth_str.starts_with("Bearer ") {
        Some(auth_str[7..].to_string())
    } else {
        None
    }
}

/// Development middleware that injects a mock authentication context
/// Returns a Context for injection in development mode
pub fn mock_auth_context() -> crate::Context {
    #[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
    return MockContext;
    
    #[cfg(feature = "oidc")]
    return Some("DEVUSER".into());
}

/// Helper function to get auth context from request extensions
pub fn get_auth_context_from_request(request: &Request) -> Option<AuthContext> {
    request
        .extensions()
        .get::<Option<AuthContext>>()
        .and_then(|ctx| ctx.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_from_cookie() {
        assert_eq!(
            extract_session_from_cookie("app_session=abc123; other=value"),
            Some("abc123".to_string())
        );

        assert_eq!(
            extract_session_from_cookie("other=value; app_session=xyz789"),
            Some("xyz789".to_string())
        );

        assert_eq!(
            extract_session_from_cookie("other=value; different=abc"),
            None
        );
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(
            extract_bearer_token("Bearer abc123token"),
            Some("abc123token".to_string())
        );

        assert_eq!(extract_bearer_token("Basic abc123"), None);

        assert_eq!(extract_bearer_token("Bearer "), Some("".to_string()));
    }
}
