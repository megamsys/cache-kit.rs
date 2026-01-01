//! # cache-kit
//!
//! A type-safe, fully generic, production-ready caching framework for Rust.
//!
//! ## Features
//!
//! - **Fully Generic:** Cache any type `T` that implements `CacheEntity`
//! - **Backend Agnostic:** Support for in-memory, Redis, Memcached, and custom backends
//! - **Database Agnostic:** Works with SQLx, tokio-postgres, Diesel, or custom repositories
//! - **Framework Independent:** Zero dependencies on web frameworks (Axum, Actix, Rocket, etc.)
//! - **Production Ready:** Built-in logging, metrics support, and error handling
//! - **Type Safe:** Compile-time verified, no magic strings
//!
//! ## Quick Start
//!
//! ### For Web Applications (Recommended)
//!
//! Use [`CacheService`] for easy sharing across threads:
//!
//! ```ignore
//! use cache_kit::{
//!     CacheService, CacheEntity, CacheFeed, DataRepository,
//!     backend::InMemoryBackend,
//!     strategy::CacheStrategy,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! // 1. Define your entity
//! #[derive(Clone, Serialize, Deserialize)]
//! struct User {
//!     id: String,
//!     name: String,
//! }
//!
//! // 2. Implement CacheEntity
//! impl CacheEntity for User {
//!     type Key = String;
//!     fn cache_key(&self) -> Self::Key { self.id.clone() }
//!     fn cache_prefix() -> &'static str { "user" }
//! }
//!
//! // 3. Create feeder
//! struct UserFeeder {
//!     id: String,
//!     user: Option<User>,
//! }
//!
//! impl CacheFeed<User> for UserFeeder {
//!     fn entity_id(&mut self) -> String { self.id.clone() }
//!     fn feed(&mut self, entity: Option<User>) { self.user = entity; }
//! }
//!
//! // 4. Create cache (wrap backend in Arc automatically)
//! let cache = CacheService::new(InMemoryBackend::new());
//!
//! // 5. Use it - CacheService is Clone for thread sharing
//! let cache_clone = cache.clone();  // Cheap - just Arc increment
//! let mut feeder = UserFeeder { id: "user_1".to_string(), user: None };
//! cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;
//! ```
//!
//! ### For Custom Patterns (Advanced)
//!
//! Use [`CacheExpander`] for explicit control:
//!
//! ```ignore
//! use cache_kit::{CacheExpander, backend::InMemoryBackend};
//! use std::sync::Arc;
//!
//! // Lower-level API - wrap in Arc yourself if needed
//! let expander = CacheExpander::new(InMemoryBackend::new());
//! let cache = Arc::new(expander);  // Manual Arc wrapping
//! let cache_clone = cache.clone();
//! ```

#[macro_use]
extern crate log;

pub mod backend;
pub mod entity;
pub mod error;
pub mod expander;
pub mod feed;
pub mod key;
pub mod observability;
pub mod repository;
pub mod serialization;
pub mod service;
pub mod strategy;

// Re-exports for convenience
pub use backend::CacheBackend;
pub use entity::CacheEntity;
pub use error::{Error, Result};
pub use expander::{CacheExpander, OperationConfig};
pub use feed::CacheFeed;
pub use repository::DataRepository;
pub use service::CacheService;
pub use strategy::CacheStrategy;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
