//! In-memory cache backend (default, thread-safe, async).
//!
//! Uses DashMap for lock-free concurrent access with per-key sharding.
//! Automatically handles TTL expiration on access.

use super::CacheBackend;
use crate::error::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

/// In-memory cache entry with optional expiration.
struct CacheEntry {
    data: Vec<u8>,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|d| Instant::now() + d);
        CacheEntry { data, expires_at }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Instant::now() > exp)
    }
}

/// Thread-safe async in-memory cache backend.
///
/// Uses DashMap for lock-free concurrent access with fine-grained per-key sharding.
/// No async locks required - operations are non-blocking.
/// Automatically handles TTL expiration on access.
///
/// # Example
///
/// ```no_run
/// use cache_kit::backend::{InMemoryBackend, CacheBackend};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = InMemoryBackend::new();
///
///     // Store data
///     backend.set("key1", b"value".to_vec(), None).await?;
///
///     // Retrieve data
///     let value = backend.get("key1").await?;
///     assert!(value.is_some());
///
///     // Store with TTL
///     backend.set("key2", b"expires".to_vec(), Some(Duration::from_secs(300))).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct InMemoryBackend {
    store: Arc<DashMap<String, CacheEntry>>,
}

impl InMemoryBackend {
    /// Create a new in-memory cache backend.
    pub fn new() -> Self {
        InMemoryBackend {
            store: Arc::new(DashMap::new()),
        }
    }

    /// Get the current number of entries in cache.
    pub async fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Get memory statistics.
    pub async fn stats(&self) -> CacheStats {
        let total_bytes: usize = self.store.iter().map(|entry| entry.data.len()).sum();
        let expired_count = self.store.iter().filter(|entry| entry.is_expired()).count();

        CacheStats {
            total_entries: self.store.len(),
            expired_entries: expired_count,
            total_bytes,
        }
    }

    /// Print cache statistics to debug log.
    pub async fn log_stats(&self) {
        let stats = self.stats().await;
        debug!(
            "Cache Stats: {} entries ({} expired), {} bytes",
            stats.total_entries, stats.expired_entries, stats.total_bytes
        );
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheBackend for InMemoryBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Check if entry exists and is not expired
        if let Some(entry) = self.store.get(key) {
            if !entry.is_expired() {
                debug!("✓ InMemory GET {} -> HIT", key);
                return Ok(Some(entry.data.clone()));
            }
        }

        // Remove expired entry if it exists
        self.store.remove(key);
        debug!("✓ InMemory GET {} -> MISS", key);
        Ok(None)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let entry = CacheEntry::new(value, ttl);
        self.store.insert(key.to_string(), entry);

        if let Some(d) = ttl {
            debug!("✓ InMemory SET {} (TTL: {:?})", key, d);
        } else {
            debug!("✓ InMemory SET {}", key);
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.store.remove(key);
        debug!("✓ InMemory DELETE {}", key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        if let Some(entry) = self.store.get(key) {
            return Ok(!entry.is_expired());
        }

        Ok(false)
    }

    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let results: Vec<Option<Vec<u8>>> = keys
            .iter()
            .map(|k| {
                if let Some(entry) = self.store.get(*k) {
                    if entry.is_expired() {
                        None
                    } else {
                        Some(entry.data.clone())
                    }
                } else {
                    None
                }
            })
            .collect();

        debug!("✓ InMemory MGET {} keys", keys.len());
        Ok(results)
    }

    async fn mdelete(&self, keys: &[&str]) -> Result<()> {
        for key in keys {
            self.store.remove(*key);
        }

        debug!("✓ InMemory MDELETE {} keys", keys.len());
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        // In-memory backend is always healthy
        Ok(true)
    }

    async fn clear_all(&self) -> Result<()> {
        self.store.clear();
        warn!("⚠ InMemory CLEAR_ALL executed - all cache cleared!");
        Ok(())
    }
}

/// Cache statistics.
#[derive(Clone, Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inmemory_backend_set_get() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");

        let result = backend.get("key1").await.expect("Failed to get");
        assert_eq!(result, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_inmemory_backend_miss() {
        let backend = InMemoryBackend::new();

        let result = backend.get("nonexistent").await.expect("Failed to get");
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_inmemory_backend_delete() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        assert!(backend
            .exists("key1")
            .await
            .expect("Failed to check exists"));

        backend.delete("key1").await.expect("Failed to delete");
        assert!(!backend
            .exists("key1")
            .await
            .expect("Failed to check exists"));
    }

    #[tokio::test]
    async fn test_inmemory_backend_ttl_expiration() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value1".to_vec(), Some(Duration::from_millis(100)))
            .await
            .expect("Failed to set");

        // Should be present immediately
        assert!(backend.get("key1").await.expect("Failed to get").is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired now
        assert!(backend.get("key1").await.expect("Failed to get").is_none());
    }

    #[tokio::test]
    async fn test_inmemory_backend_mget() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set");

        let results = backend
            .mget(&["key1", "key2", "key3"])
            .await
            .expect("Failed to mget");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Some(b"value1".to_vec()));
        assert_eq!(results[1], Some(b"value2".to_vec()));
        assert_eq!(results[2], None);
    }

    #[tokio::test]
    async fn test_inmemory_backend_mdelete() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("key3", b"value3".to_vec(), None)
            .await
            .expect("Failed to set");

        assert_eq!(backend.len().await, 3);

        backend
            .mdelete(&["key1", "key2"])
            .await
            .expect("Failed to mdelete");

        assert_eq!(backend.len().await, 1);
        assert!(backend.get("key3").await.expect("Failed to get").is_some());
    }

    #[tokio::test]
    async fn test_inmemory_backend_clear_all() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set");

        assert_eq!(backend.len().await, 2);

        backend.clear_all().await.expect("Failed to clear");

        assert_eq!(backend.len().await, 0);
    }

    #[tokio::test]
    async fn test_inmemory_backend_stats() {
        let backend = InMemoryBackend::new();

        backend
            .set("key1", b"value_with_data".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("key2", b"data".to_vec(), None)
            .await
            .expect("Failed to set");

        let stats = backend.stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.expired_entries, 0);
        assert!(stats.total_bytes > 0);
    }

    #[tokio::test]
    async fn test_inmemory_backend_clone() {
        let backend1 = InMemoryBackend::new();
        backend1
            .set("key", b"value".to_vec(), None)
            .await
            .expect("Failed to set");

        let backend2 = backend1.clone();

        // Both backends share the same store
        let value = backend2.store.get("key").map(|e| e.data.clone());
        assert_eq!(value, Some(b"value".to_vec()));
    }

    #[tokio::test]
    async fn test_inmemory_backend_thread_safe() {
        use std::sync::Arc;

        let backend = Arc::new(InMemoryBackend::new());
        let mut handles = vec![];

        for i in 0..10 {
            let backend_clone = Arc::clone(&backend);
            let handle = tokio::spawn(async move {
                let b = (*backend_clone).clone();
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);
                b.set(&key, value.into_bytes(), None)
                    .await
                    .expect("Failed to set");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task failed");
        }

        assert!(backend.clone().len().await >= 10);
    }
}
