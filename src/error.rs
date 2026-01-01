//! Error types for the cache framework.

use std::fmt;

/// Result type for cache operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for cache framework.
///
/// All cache operations return `Result<T>` where `Result` is defined as `std::result::Result<T, Error>`.
/// Different error variants represent different failure modes:
#[derive(Debug, Clone)]
pub enum Error {
    /// Serialization failed when converting entity to cache bytes.
    ///
    /// This occurs when the entity's `Serde` implementation fails.
    /// Common causes:
    /// - Entity contains non-serializable types
    /// - Serde serialization panic/error
    /// - Postcard codec error
    SerializationError(String),

    /// Deserialization failed when converting cache bytes to entity.
    ///
    /// This indicates corrupted or malformed data in cache.
    /// Common causes:
    /// - Cache was corrupted during transport or storage
    /// - Invalid Postcard encoding
    /// - Incomplete data read from backend
    ///
    /// **Recovery:** Cache entry should be evicted and recomputed.
    DeserializationError(String),

    /// Validation failed in feeder or entity.
    ///
    /// This is raised when:
    /// - `CacheFeed::validate()` returns an error
    /// - `CacheEntity::validate()` returns an error after deserialization
    ///
    /// Implement these methods to add custom validation logic.
    ValidationError(String),

    /// Cache miss: key not found in cache.
    ///
    /// Not necessarily an error condition, but indicates cache entry was absent.
    /// Only returned with `CacheStrategy::Fresh` when cache lookup fails.
    CacheMiss,

    /// Backend storage error (Redis, Memcached, etc).
    ///
    /// This indicates the cache backend is unavailable or returned an error.
    /// Common causes:
    /// - Redis/Memcached connection lost
    /// - Network timeout
    /// - Backend storage full
    /// - Backend protocol error
    ///
    /// **Recovery:** Retry the operation or fallback to database.
    BackendError(String),

    /// Data repository error (database, etc).
    ///
    /// This indicates the source repository (database) failed to fetch data.
    /// Common causes:
    /// - Database connection lost
    /// - Query syntax error
    /// - Database server error
    /// - Row/record not found
    ///
    /// **Recovery:** Retry after connection recovery.
    RepositoryError(String),

    /// Operation exceeded configured timeout threshold.
    ///
    /// This occurs when cache or repository operations take too long.
    /// Common causes:
    /// - Network latency
    /// - Slow database query
    /// - Backend overload
    ///
    /// **Recovery:** Retry with exponential backoff.
    Timeout(String),

    /// Configuration error during crate initialization.
    ///
    /// This occurs when creating backends or policies with invalid config.
    /// Common causes:
    /// - Invalid connection string
    /// - Missing required configuration
    /// - Invalid TTL policy
    ///
    /// **Recovery:** Fix configuration and restart.
    ConfigError(String),

    /// Feature not implemented or not enabled.
    ///
    /// This indicates a requested feature is not available.
    /// Common causes:
    /// - Cargo feature not enabled (e.g., "redis" for RedisBackend)
    /// - Backend-specific operation called on wrong backend type
    ///
    /// **Recovery:** Enable the required Cargo feature.
    NotImplemented(String),

    /// Invalid cache entry: corrupted envelope or bad magic.
    ///
    /// This indicates the cache entry header is invalid.
    /// Returned when:
    /// - Magic header is not `b"CKIT"`
    /// - Envelope deserialization fails
    /// - Non-cache-kit data in cache key
    ///
    /// **Recovery:** Evict the cache entry and recompute.
    InvalidCacheEntry(String),

    /// Schema version mismatch between code and cached data.
    ///
    /// This indicates the cache entry was created with a different schema version.
    /// Raised when:
    /// - `CURRENT_SCHEMA_VERSION` changed
    /// - Struct fields were added/removed/reordered
    /// - Enum variants changed
    ///
    /// **Recovery:** Cache entry is automatically evicted and recomputed on next access.
    /// No action needed - this is expected during deployments.
    VersionMismatch {
        /// Expected schema version (from compiled code)
        expected: u32,
        /// Found schema version (from cached entry)
        found: u32,
    },

    /// Generic error with custom message.
    ///
    /// Used for errors that don't fit into other variants.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Error::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            Error::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Error::CacheMiss => write!(f, "Cache miss"),
            Error::BackendError(msg) => write!(f, "Backend error: {}", msg),
            Error::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            Error::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Error::ConfigError(msg) => write!(f, "Config error: {}", msg),
            Error::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            Error::InvalidCacheEntry(msg) => {
                write!(f, "Invalid cache entry: {}", msg)
            }
            Error::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "Cache version mismatch: expected {}, found {}",
                    expected, found
                )
            }
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

// ============================================================================
// Conversions from other error types
// ============================================================================

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        if e.is_io() {
            Error::BackendError(e.to_string())
        } else if e.is_syntax() {
            Error::DeserializationError(e.to_string())
        } else {
            Error::SerializationError(e.to_string())
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::BackendError(e.to_string())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Other(e)
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::Other(e.to_string())
    }
}

#[cfg(feature = "redis")]
impl From<redis::RedisError> for Error {
    fn from(e: redis::RedisError) -> Self {
        Error::BackendError(format!("Redis error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::ValidationError("Test".to_string());
        assert_eq!(err.to_string(), "Validation error: Test");
    }

    #[test]
    fn test_error_from_string() {
        let err: Error = "test error".into();
        assert!(matches!(err, Error::Other(_)));
    }
}
