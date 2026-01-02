use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use cache_kit::{
    backend::InMemoryBackend, strategy::CacheStrategy, CacheEntity, CacheFeed, DataRepository,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

mod metrics;

use metrics::PrometheusMetrics;

/// User entity
#[derive(Clone, Serialize, Deserialize, Debug)]
struct User {
    id: String,
    name: String,
    email: String,
}

impl CacheEntity for User {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn cache_prefix() -> &'static str {
        "user"
    }
}

/// User feeder for cache
struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<User>) {
        self.user = entity;
    }
}

/// Mock user repository
struct UserRepository;

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::error::Result<Option<User>> {
        // Simulate database fetch with some delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let user = match id.as_str() {
            "user_001" => Some(User {
                id: id.clone(),
                name: "Alice Johnson".to_string(),
                email: "alice@example.com".to_string(),
            }),
            "user_002" => Some(User {
                id: id.clone(),
                name: "Bob Smith".to_string(),
                email: "bob@example.com".to_string(),
            }),
            "user_003" => Some(User {
                id: id.clone(),
                name: "Charlie Brown".to_string(),
                email: "charlie@example.com".to_string(),
            }),
            _ => None,
        };

        Ok(user)
    }
}

/// Application state
#[derive(Clone)]
struct AppState {
    cache: Arc<cache_kit::CacheExpander<InMemoryBackend>>,
    metrics: Arc<PrometheusMetrics>,
}

/// Get user by ID with caching
/// Metrics are automatically recorded by the cache-kit library via CacheMetrics trait
async fn get_user(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let cache = &state.cache;

    let mut feeder = UserFeeder {
        id: id.clone(),
        user: None,
    };

    let repository = UserRepository;

    match cache
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await
    {
        Ok(_) => {
            if let Some(user) = feeder.user {
                (StatusCode::OK, Json(user)).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "User not found"})),
                )
                    .into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Cache error: {}", e)})),
        )
            .into_response(),
    }
}

/// Metrics endpoint
async fn metrics_handler(State(state): State<AppState>) -> String {
    state.metrics.render_prometheus()
}

/// Health check endpoint
async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "cache-kit-axum-example"
    }))
}

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();

    // Initialize cache with metrics
    let backend = InMemoryBackend::new();
    let metrics = Arc::new(PrometheusMetrics::new());
    let cache =
        Arc::new(cache_kit::CacheExpander::new(backend).with_metrics(Box::new((*metrics).clone())));

    let state = AppState { cache, metrics };

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/user/{id}", get(get_user))
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind port 3000");

    println!("Server running on http://127.0.0.1:3000");
    println!("API endpoint: http://127.0.0.1:3000/api/user/user_001");
    println!("Metrics endpoint: http://127.0.0.1:3000/metrics");
    println!("Health check: http://127.0.0.1:3000/health");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
