pub mod cache_config;
pub mod db;
pub mod feeders;
pub mod grpc;
pub mod models;
pub mod repository;

pub use grpc::start_grpc_server;

use cache_kit::backend::InMemoryBackend;
use cache_kit::CacheService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub cache_service: CacheService<InMemoryBackend>,
}

impl AppState {
    pub fn new(db: sqlx::PgPool, cache_backend: Arc<InMemoryBackend>) -> Self {
        // Create cache service from the backend
        // Clone is cheap since InMemoryBackend uses Arc internally
        let cache_service = CacheService::new(cache_backend.as_ref().clone());
        Self { db, cache_service }
    }
}
