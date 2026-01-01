//! Cache backend implementations.

use crate::error::Result;
use std::time::Duration;

pub mod inmemory;
#[cfg(feature = "memcached")]
pub mod memcached;
#[cfg(feature = "redis")]
pub mod redis;

pub use inmemory::InMemoryBackend;
#[cfg(feature = "memcached")]
pub use memcached::{MemcachedBackend, MemcachedConfig};
#[cfg(feature = "redis")]
pub use redis::{PoolStats, RedisBackend, RedisConfig};

/// Trait for cache backend implementations.
///
/// Abstracts storage operations, allowing swappable backends.
/// Implementations: InMemory (default), Redis, Memcached, RocksDB, Database, S3, etc.
///
/// **IMPORTANT:** All methods use `&self` instead of `&mut self` to allow concurrent access.
/// Backend implementations should use interior mutability (RwLock, Mutex, or external storage).
///
/// **ASYNC:** All methods are async and must be awaited.
#[allow(async_fn_in_trait)]
pub trait CacheBackend: Send + Sync + Clone {
    /// Retrieve value from cache by key.
    ///
    /// # Returns
    /// - `Ok(Some(bytes))` - Value found in cache
    /// - `Ok(None)` - Cache miss (key not found)
    ///
    /// # Errors
    /// Returns `Err` if backend error occurs (connection lost, etc.)
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Store value in cache with optional TTL.
    ///
    /// # Arguments
    /// - `key`: Cache key
    /// - `value`: Serialized entity bytes
    /// - `ttl`: Time-to-live. None = use backend default or infinite
    ///
    /// # Errors
    /// Returns `Err` if backend error occurs
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;

    /// Remove value from cache.
    ///
    /// # Errors
    /// Returns `Err` if backend error occurs
    async fn delete(&self, key: &str) -> Result<()>;

    /// Check if key exists in cache (optional optimization).
    ///
    /// # Errors
    /// Returns `Err` if backend error occurs
    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    /// Bulk get operation (optional optimization).
    ///
    /// Default implementation calls `get()` for each key.
    /// Override for batch efficiency (e.g., Redis MGET).
    ///
    /// # Errors
    /// Returns `Err` if backend error occurs
    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    /// Bulk delete operation (optional optimization).
    ///
    /// Default implementation calls `delete()` for each key.
    /// Override for batch efficiency (e.g., Redis DEL).
    ///
    /// # Errors
    /// Returns `Err` if backend error occurs
    async fn mdelete(&self, keys: &[&str]) -> Result<()> {
        for key in keys {
            self.delete(key).await?;
        }
        Ok(())
    }

    /// Health check - verify backend is accessible.
    ///
    /// Used for readiness probes, circuit breakers, etc.
    ///
    /// # Errors
    /// Returns `Err` if backend is not accessible
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    /// Optional: Clear all cache (use with caution).
    ///
    /// # Errors
    /// Returns `Err` if operation is not implemented or fails
    async fn clear_all(&self) -> Result<()> {
        Err(crate::error::Error::NotImplemented(
            "clear_all not implemented for this backend".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_exists_default() {
        let backend = InMemoryBackend::new();
        backend
            .set("key", vec![1, 2, 3], None)
            .await
            .expect("Failed to set key");
        assert!(backend.exists("key").await.expect("Failed to check exists"));
        assert!(!backend
            .exists("nonexistent")
            .await
            .expect("Failed to check exists"));
    }
}
