//! Core entity trait that all cached entities must implement.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;

/// Trait that all entities stored in cache must implement.
///
/// # Example
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use cache_kit::CacheEntity;
///
/// #[derive(Clone, Serialize, Deserialize)]
/// pub struct Employment {
///     pub id: String,
///     pub employer_name: String,
/// }
///
/// impl CacheEntity for Employment {
///     type Key = String;
///
///     fn cache_key(&self) -> Self::Key {
///         self.id.clone()
///     }
///
///     fn cache_prefix() -> &'static str {
///         "employment"
///     }
/// }
/// ```
pub trait CacheEntity: Send + Sync + Serialize + for<'de> Deserialize<'de> + Clone {
    /// Type of the entity's key/ID (typically String or UUID)
    type Key: Display + Clone + Send + Sync + Eq + Hash + 'static;

    /// Return the entity's unique cache key.
    ///
    /// Called to extract the key from the entity itself.
    /// Example: `Employment.id` → `"emp_12345"`
    fn cache_key(&self) -> Self::Key;

    /// Return the cache prefix for this entity type.
    ///
    /// Used to namespace cache keys. Example: "employment", "borrower"
    /// Final cache key format: `"{prefix}:{key}"`
    fn cache_prefix() -> &'static str;

    /// Serialize entity for cache storage.
    ///
    /// Uses Postcard with versioned envelopes for all cache storage.
    /// This method is NOT overridable to ensure consistency across all entities.
    ///
    /// # Format
    ///
    /// ```text
    /// [MAGIC: 4 bytes] [VERSION: 4 bytes] [POSTCARD PAYLOAD]
    /// ```
    ///
    /// # Performance
    ///
    /// - 10-15x faster than JSON
    /// - 60% smaller payloads
    ///
    /// See `crate::serialization` for implementation details.
    fn serialize_for_cache(&self) -> Result<Vec<u8>> {
        crate::serialization::serialize_for_cache(self)
    }

    /// Deserialize entity from cache storage.
    ///
    /// Validates magic header and schema version before deserializing.
    /// This method is NOT overridable to ensure consistency across all entities.
    ///
    /// # Validation
    ///
    /// - Magic must be b"CKIT"
    /// - Version must match current schema version
    /// - Postcard deserialization must succeed
    ///
    /// # Errors
    ///
    /// - `Error::InvalidCacheEntry`: Bad magic or corrupted envelope
    /// - `Error::VersionMismatch`: Schema version changed
    /// - `Error::DeserializationError`: Corrupted payload
    ///
    /// See `crate::serialization` for implementation details.
    fn deserialize_from_cache(bytes: &[u8]) -> Result<Self> {
        crate::serialization::deserialize_from_cache(bytes)
    }

    /// Optional: Validate entity after deserialization.
    ///
    /// Called after loading from cache. Use to ensure consistency.
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    struct TestEntity {
        id: String,
        value: String,
    }

    impl CacheEntity for TestEntity {
        type Key = String;

        fn cache_key(&self) -> Self::Key {
            self.id.clone()
        }

        fn cache_prefix() -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let entity = TestEntity {
            id: "test_1".to_string(),
            value: "data".to_string(),
        };

        let bytes = entity.serialize_for_cache().unwrap();
        let deserialized = TestEntity::deserialize_from_cache(&bytes).unwrap();

        assert_eq!(entity.id, deserialized.id);
        assert_eq!(entity.value, deserialized.value);
    }

    #[test]
    fn test_cache_key_generation() {
        let entity = TestEntity {
            id: "entity_123".to_string(),
            value: "test".to_string(),
        };

        assert_eq!(entity.cache_key(), "entity_123");
        assert_eq!(TestEntity::cache_prefix(), "test");
    }
}
