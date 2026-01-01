use cache_kit::CacheEntity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, FromRow)]
pub struct User {
    pub id: Uuid, // UUIDv7 generated in Rust
    pub username: String,
    pub email: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new User with auto-generated UUIDv7
    pub fn new(username: String, email: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(), // ✅ UUIDv7 generation in Rust (Option 1)
            username,
            email,
            created_at: now,
            updated_at: now,
        }
    }
}

impl CacheEntity for User {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.to_string()
    }

    fn cache_prefix() -> &'static str {
        "user"
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, FromRow)]
pub struct Product {
    pub id: Uuid, // UUIDv7 generated in Rust
    pub name: String,
    pub price: i64, // Price in cents (e.g., 9999 = $99.99)
    pub stock: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

impl Product {
    /// Create a new Product with auto-generated UUIDv7
    /// Price should be in cents (e.g., 9999 for $99.99)
    pub fn new(name: String, price: i64, stock: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(), // ✅ UUIDv7 generation in Rust (Option 1)
            name,
            price,
            stock,
            created_at: now,
            updated_at: now,
        }
    }
}

impl CacheEntity for Product {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.to_string()
    }

    fn cache_prefix() -> &'static str {
        "product"
    }
}
