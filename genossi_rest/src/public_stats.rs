use std::time::{Duration, Instant};

use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::Json;
use serde::Serialize;
use tokio::sync::RwLock;
use utoipa::ToSchema;

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

pub struct PublicStatsCache {
    config_enabled: RwLock<Option<(bool, Instant)>>,
    member_count: RwLock<Option<(u64, Instant)>>,
}

impl Default for PublicStatsCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PublicStatsCache {
    pub fn new() -> Self {
        Self {
            config_enabled: RwLock::new(None),
            member_count: RwLock::new(None),
        }
    }

    pub async fn get_config(&self) -> Option<bool> {
        let guard = self.config_enabled.read().await;
        guard.and_then(|(val, cached_at)| {
            if cached_at.elapsed() < CACHE_TTL {
                Some(val)
            } else {
                None
            }
        })
    }

    pub async fn set_config(&self, enabled: bool) {
        let mut guard = self.config_enabled.write().await;
        *guard = Some((enabled, Instant::now()));
    }

    pub async fn get_count(&self) -> Option<u64> {
        let guard = self.member_count.read().await;
        guard.and_then(|(val, cached_at)| {
            if cached_at.elapsed() < CACHE_TTL {
                Some(val)
            } else {
                None
            }
        })
    }

    pub async fn set_count(&self, count: u64) {
        let mut guard = self.member_count.write().await;
        *guard = Some((count, Instant::now()));
    }
}

#[derive(Serialize, ToSchema)]
pub struct MemberCountResponse {
    pub count: u64,
}

pub trait PublicStatsState: Clone + Send + Sync + 'static {
    fn public_stats_cache(&self) -> &PublicStatsCache;
    fn get_public_stats_enabled(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<bool>> + Send + '_>>;
    fn get_active_member_count(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<u64>> + Send + '_>>;
}

#[utoipa::path(
    get,
    path = "/api/public/member-count",
    responses(
        (status = 200, description = "Active member count", body = MemberCountResponse),
        (status = 403, description = "Public stats not enabled"),
    ),
    tag = "Public Stats"
)]
async fn get_member_count<S: PublicStatsState>(State(state): State<S>) -> Response {
    let cache = state.public_stats_cache();

    // Check config (cached)
    let enabled = match cache.get_config().await {
        Some(val) => val,
        None => {
            let val = state.get_public_stats_enabled().await.unwrap_or(false);
            cache.set_config(val).await;
            val
        }
    };

    if !enabled {
        return Response::builder()
            .status(403)
            .body(axum::body::Body::from("Public stats not enabled"))
            .unwrap()
            .into_response();
    }

    // Get count (cached)
    let count = match cache.get_count().await {
        Some(val) => val,
        None => {
            let val = state.get_active_member_count().await.unwrap_or(0);
            cache.set_count(val).await;
            val
        }
    };

    Json(MemberCountResponse { count }).into_response()
}

pub fn generate_route<S: PublicStatsState>() -> axum::Router<S> {
    axum::Router::new().route("/member-count", get(get_member_count::<S>))
}

#[derive(utoipa::OpenApi)]
#[openapi(paths(get_member_count), components(schemas(MemberCountResponse)))]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_fresh_value() {
        let cache = PublicStatsCache::new();
        assert!(cache.get_config().await.is_none());
        assert!(cache.get_count().await.is_none());

        cache.set_config(true).await;
        cache.set_count(42).await;

        assert_eq!(cache.get_config().await, Some(true));
        assert_eq!(cache.get_count().await, Some(42));
    }

    #[tokio::test]
    async fn test_cache_overwrite() {
        let cache = PublicStatsCache::new();
        cache.set_count(10).await;
        assert_eq!(cache.get_count().await, Some(10));

        cache.set_count(20).await;
        assert_eq!(cache.get_count().await, Some(20));
    }

    #[tokio::test]
    async fn test_cache_config_false() {
        let cache = PublicStatsCache::new();
        cache.set_config(false).await;
        assert_eq!(cache.get_config().await, Some(false));
    }
}
