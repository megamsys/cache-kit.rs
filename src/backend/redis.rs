//! Redis cache backend implementation.

use super::CacheBackend;
use crate::error::{Error, Result};
use deadpool_redis::{redis::AsyncCommands, Config as PoolConfig, Pool, Runtime};
use std::time::Duration;

/// Pool statistics information.
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub connections: u32,
    pub idle_connections: u32,
}

/// Default Redis connection pool size.
/// Formula: (CPU cores × 2) + 1
/// For 8-core systems: 16 connections is optimal
/// Override with REDIS_POOL_SIZE environment variable
const DEFAULT_POOL_SIZE: u32 = 16;

/// Configuration for Redis backend.
#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: u32,
    pub pool_size: u32,
    pub connection_timeout: Duration,
}

impl Default for RedisConfig {
    fn default() -> Self {
        RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            username: None,
            password: None,
            database: 0,
            pool_size: DEFAULT_POOL_SIZE,
            connection_timeout: Duration::from_secs(5),
        }
    }
}

impl RedisConfig {
    /// Build Redis connection string.
    pub fn connection_string(&self) -> String {
        if let Some(password) = &self.password {
            if let Some(username) = &self.username {
                format!(
                    "redis://{}:{}@{}:{}/{}",
                    username, password, self.host, self.port, self.database
                )
            } else {
                format!(
                    "redis://default:{}@{}:{}/{}",
                    password, self.host, self.port, self.database
                )
            }
        } else {
            format!("redis://{}:{}/{}", self.host, self.port, self.database)
        }
    }
}

/// Redis backend with connection pooling and async operations.
///
/// Uses deadpool for efficient async resource management and pooling.
///
/// # Example
///
/// ```no_run
/// # use cache_kit::backend::{RedisBackend, RedisConfig, CacheBackend};
/// # use cache_kit::error::Result;
/// # async fn example() -> Result<()> {
/// let config = RedisConfig::default();
/// let mut backend = RedisBackend::new(config).await?;
///
/// backend.set("key", b"value".to_vec(), None).await?;
/// let value = backend.get("key").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RedisBackend {
    pool: Pool,
}

impl RedisBackend {
    /// Create new Redis backend from configuration.
    ///
    /// # Errors
    /// Returns `Err` if pool creation fails or connection cannot be established.
    pub async fn new(config: RedisConfig) -> Result<Self> {
        let conn_str = config.connection_string();
        let mut cfg = PoolConfig::from_url(conn_str);
        cfg.pool = Some(deadpool_redis::PoolConfig::new(config.pool_size as usize));

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| Error::BackendError(format!("Failed to create Redis pool: {}", e)))?;

        info!(
            "✓ Redis backend initialized: {}:{}",
            config.host, config.port
        );

        Ok(RedisBackend { pool })
    }

    /// Create from connection string directly.
    ///
    /// Pool size is determined by:
    /// 1. `REDIS_POOL_SIZE` environment variable (if set)
    /// 2. `DEFAULT_POOL_SIZE` constant (16)
    ///
    /// # Errors
    /// Returns `Err` if pool creation fails or connection cannot be established.
    pub async fn from_connection_string(conn_str: &str) -> Result<Self> {
        let pool_size = std::env::var("REDIS_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(DEFAULT_POOL_SIZE);

        let mut cfg = PoolConfig::from_url(conn_str);
        cfg.pool = Some(deadpool_redis::PoolConfig::new(pool_size as usize));

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| Error::BackendError(format!("Failed to create Redis pool: {}", e)))?;

        info!(
            "✓ Redis backend initialized from connection string (pool size: {})",
            pool_size
        );

        Ok(RedisBackend { pool })
    }

    /// Get current pool statistics.
    pub fn pool_stats(&self) -> PoolStats {
        let status = self.pool.status();
        PoolStats {
            connections: status.size as u32,
            idle_connections: status.available as u32,
        }
    }
}

impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        let value: Option<Vec<u8>> = conn
            .get(key)
            .await
            .map_err(|e| Error::BackendError(format!("Redis GET failed for key {}: {}", key, e)))?;

        if value.is_some() {
            debug!("✓ Redis GET {} -> HIT", key);
        } else {
            debug!("✓ Redis GET {} -> MISS", key);
        }

        Ok(value)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        match ttl {
            Some(duration) => {
                let seconds = duration.as_secs();
                conn.set_ex::<_, _, ()>(key, value, seconds)
                    .await
                    .map_err(|e| {
                        Error::BackendError(format!("Redis SET_EX failed for key {}: {}", key, e))
                    })?;
                debug!("✓ Redis SET {} (TTL: {}s)", key, seconds);
            }
            None => {
                conn.set::<_, _, ()>(key, value).await.map_err(|e| {
                    Error::BackendError(format!("Redis SET failed for key {}: {}", key, e))
                })?;
                debug!("✓ Redis SET {}", key);
            }
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        conn.del::<_, ()>(key)
            .await
            .map_err(|e| Error::BackendError(format!("Redis DEL failed for key {}: {}", key, e)))?;

        debug!("✓ Redis DELETE {}", key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        let exists: bool = conn.exists(key).await.map_err(|e| {
            Error::BackendError(format!("Redis EXISTS failed for key {}: {}", key, e))
        })?;

        Ok(exists)
    }

    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        let values: Vec<Option<Vec<u8>>> = conn
            .get(keys)
            .await
            .map_err(|e| Error::BackendError(format!("Redis MGET failed: {}", e)))?;

        debug!("✓ Redis MGET {} keys", keys.len());
        Ok(values)
    }

    async fn mdelete(&self, keys: &[&str]) -> Result<()> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        conn.del::<_, ()>(keys)
            .await
            .map_err(|e| Error::BackendError(format!("Redis DEL (bulk) failed: {}", e)))?;

        debug!("✓ Redis MDELETE {} keys", keys.len());
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        // Use deadpool_redis::redis::cmd for PING command
        let pong: String = deadpool_redis::redis::cmd("PING")
            .query_async(&mut *conn)
            .await
            .map_err(|e| Error::BackendError(format!("Redis PING failed: {}", e)))?;

        Ok(pong == "PONG" || pong.contains("PONG"))
    }

    async fn clear_all(&self) -> Result<()> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                Error::BackendError(format!("Failed to get Redis connection: {}", e))
            })?;

        deadpool_redis::redis::cmd("FLUSHDB")
            .query_async::<()>(&mut *conn)
            .await
            .map_err(|e| Error::BackendError(format!("Redis FLUSHDB failed: {}", e)))?;

        warn!("⚠ Redis FLUSHDB executed - all cache cleared!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_config_connection_string() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: Some("password".to_string()),
            username: Some("user".to_string()),
            database: 0,
            pool_size: 10,
            connection_timeout: Duration::from_secs(5),
        };

        assert_eq!(
            config.connection_string(),
            "redis://user:password@localhost:6379/0"
        );
    }

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 6379);
        assert_eq!(config.database, 0);
        assert_eq!(config.pool_size, DEFAULT_POOL_SIZE);
    }

    #[test]
    fn test_redis_config_no_auth() {
        let config = RedisConfig::default();
        assert_eq!(config.connection_string(), "redis://localhost:6379/0");
    }

    #[test]
    fn test_redis_config_custom_timeout() {
        let timeout = Duration::from_secs(10);
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            username: None,
            password: None,
            database: 0,
            pool_size: 16,
            connection_timeout: timeout,
        };

        assert_eq!(config.connection_timeout, timeout);
    }

    // Integration tests - require running Redis server
    // Uncomment and run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_new() {
        let config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            username: None,
            password: None,
            database: 0,
            pool_size: 16,
            connection_timeout: Duration::from_secs(5),
        };

        let result = RedisBackend::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_from_connection_string() {
        let result = RedisBackend::from_connection_string("redis://localhost:6379/0").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_set_get() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set("test_key", b"test_value".to_vec(), None)
            .await
            .expect("Failed to set");

        let result = backend.get("test_key").await.expect("Failed to get");
        assert_eq!(result, Some(b"test_value".to_vec()));
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_get_miss() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        let result = backend.get("nonexistent_key").await.expect("Failed to get");
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_delete() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set("delete_key", b"value".to_vec(), None)
            .await
            .expect("Failed to set");

        backend
            .delete("delete_key")
            .await
            .expect("Failed to delete");

        let result = backend.get("delete_key").await.expect("Failed to get");
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_exists() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set("exists_key", b"value".to_vec(), None)
            .await
            .expect("Failed to set");

        let exists = backend
            .exists("exists_key")
            .await
            .expect("Failed to check exists");
        assert!(exists);

        let not_exists = backend
            .exists("nonexistent")
            .await
            .expect("Failed to check exists");
        assert!(!not_exists);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_mget() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set("mget_key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("mget_key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set");

        let results = backend
            .mget(&["mget_key1", "mget_key2", "nonexistent"])
            .await
            .expect("Failed to mget");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Some(b"value1".to_vec()));
        assert_eq!(results[1], Some(b"value2".to_vec()));
        assert_eq!(results[2], None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_mdelete() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set("mdelete_key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("mdelete_key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set");

        backend
            .mdelete(&["mdelete_key1", "mdelete_key2"])
            .await
            .expect("Failed to mdelete");

        let result1 = backend.get("mdelete_key1").await.expect("Failed to get");
        let result2 = backend.get("mdelete_key2").await.expect("Failed to get");
        assert_eq!(result1, None);
        assert_eq!(result2, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_ttl() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set(
                "ttl_key",
                b"expires_soon".to_vec(),
                Some(Duration::from_secs(2)),
            )
            .await
            .expect("Failed to set");

        let result = backend.get("ttl_key").await.expect("Failed to get");
        assert_eq!(result, Some(b"expires_soon".to_vec()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(3)).await;

        let expired = backend.get("ttl_key").await.expect("Failed to get");
        assert_eq!(expired, None);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_health_check() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        let healthy = backend
            .health_check()
            .await
            .expect("Failed to check health");
        assert!(healthy);
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_clear_all() {
        let backend = RedisBackend::from_connection_string("redis://localhost:6379/0")
            .await
            .expect("Failed to create backend");

        backend
            .set("clear_key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set");
        backend
            .set("clear_key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set");

        backend.clear_all().await.expect("Failed to clear");

        let result1 = backend.get("clear_key1").await.expect("Failed to get");
        let result2 = backend.get("clear_key2").await.expect("Failed to get");
        assert_eq!(result1, None);
        assert_eq!(result2, None);
    }
}
