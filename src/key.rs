//! Cache key management utilities.

use crate::entity::CacheEntity;

/// Type alias for key generator function.
type KeyGeneratorFn = dyn Fn(&dyn std::fmt::Display) -> String + Send + Sync;

/// Builder for cache keys.
pub struct CacheKeyBuilder;

impl CacheKeyBuilder {
    /// Build full cache key from entity type and ID.
    pub fn build<T: CacheEntity>(id: &T::Key) -> String {
        format!("{}:{}", T::cache_prefix(), id)
    }

    /// Build cache key with custom prefix.
    pub fn build_with_prefix(prefix: &str, id: &dyn std::fmt::Display) -> String {
        format!("{}:{}", prefix, id)
    }

    /// Build composite key from multiple parts.
    pub fn build_composite(parts: &[&str]) -> String {
        parts.join(":")
    }

    /// Parse a composite key into parts.
    pub fn parse(key: &str) -> Vec<&str> {
        key.split(':').collect()
    }
}

/// Registry for custom cache key generators.
pub struct KeyRegistry {
    generators: std::collections::HashMap<String, Box<KeyGeneratorFn>>,
}

impl KeyRegistry {
    pub fn new() -> Self {
        KeyRegistry {
            generators: std::collections::HashMap::new(),
        }
    }

    /// Register a custom key generator for a type.
    pub fn register<F>(&mut self, type_name: String, generator: F)
    where
        F: Fn(&dyn std::fmt::Display) -> String + Send + Sync + 'static,
    {
        self.generators.insert(type_name, Box::new(generator));
    }

    /// Get a key generator for a type.
    pub fn get(&self, type_name: &str) -> Option<&KeyGeneratorFn> {
        self.generators.get(type_name).map(|b| b.as_ref())
    }

    /// Generate key using registered generator.
    pub fn generate(&self, type_name: &str, id: &dyn std::fmt::Display) -> Option<String> {
        self.get(type_name).map(|gen| gen(id))
    }
}

impl Default for KeyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    struct TestEntity {
        id: String,
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
    fn test_cache_key_builder() {
        let key = CacheKeyBuilder::build::<TestEntity>(&"entity_123".to_string());
        assert_eq!(key, "test:entity_123");
    }

    #[test]
    fn test_cache_key_builder_custom_prefix() {
        let key = CacheKeyBuilder::build_with_prefix("custom", &"123");
        assert_eq!(key, "custom:123");
    }

    #[test]
    fn test_composite_key_builder() {
        let key = CacheKeyBuilder::build_composite(&["user", "123", "profile"]);
        assert_eq!(key, "user:123:profile");
    }

    #[test]
    fn test_composite_key_parser() {
        let key = "user:123:profile";
        let parts = CacheKeyBuilder::parse(key);
        assert_eq!(parts, vec!["user", "123", "profile"]);
    }

    #[test]
    fn test_key_registry() {
        let mut registry = KeyRegistry::new();

        registry.register("custom".to_string(), |id| format!("CUSTOM_{}", id));

        let generated = registry.generate("custom", &"123").unwrap();
        assert_eq!(generated, "CUSTOM_123");

        assert!(registry.generate("unknown", &"123").is_none());
    }
}
