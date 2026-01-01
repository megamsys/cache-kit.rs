//! Feeder trait for consuming cached data.

use crate::entity::CacheEntity;
use crate::error::Result;

/// Generic trait for consuming cached data from operations.
///
/// Replaces specific feeder traits with a single generic abstraction.
///
/// # Example
///
/// ```no_run
/// use cache_kit::{CacheFeed, CacheEntity};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct Employment {
///     id: String,
///     name: String,
/// }
///
/// impl CacheEntity for Employment {
///     type Key = String;
///     fn cache_key(&self) -> Self::Key { self.id.clone() }
///     fn cache_prefix() -> &'static str { "employment" }
/// }
///
/// struct EmploymentFeeder {
///     id: String,
///     employment: Option<Employment>,
/// }
///
/// impl CacheFeed<Employment> for EmploymentFeeder {
///     fn entity_id(&mut self) -> String {
///         self.id.clone()
///     }
///
///     fn feed(&mut self, entity: Option<Employment>) {
///         self.employment = entity;
///     }
/// }
/// ```
pub trait CacheFeed<T: CacheEntity>: Send {
    /// Return the entity ID to fetch cache for.
    ///
    /// Called first by expander to determine which cache entry to fetch.
    fn entity_id(&mut self) -> T::Key;

    /// Feed the loaded entity into this feeder.
    ///
    /// Called by expander after successfully loading from cache.
    /// The feeder stores the entity internally for later use.
    fn feed(&mut self, entity: Option<T>);

    /// Optional: Validate the feeder before processing.
    ///
    /// Called before attempting cache fetch. Use to validate state.
    /// Example: Check that entity_id is not empty
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// Optional: Called after entity is loaded but before returning.
    ///
    /// Useful for post-processing, logging, or metrics.
    fn on_loaded(&mut self, _entity: &T) -> Result<()> {
        Ok(())
    }

    /// Optional: Called when cache miss occurs.
    ///
    /// Useful for metrics or custom behavior.
    fn on_miss(&mut self, _key: &str) -> Result<()> {
        Ok(())
    }

    /// Optional: Called when cache hit occurs.
    ///
    /// Useful for metrics or logging.
    fn on_hit(&mut self, _key: &str) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// Generic Feeder Implementations
// ============================================================================

/// Generic feeder for single entities.
pub struct GenericFeeder<T: CacheEntity> {
    pub id: T::Key,
    pub data: Option<T>,
}

impl<T: CacheEntity> GenericFeeder<T> {
    pub fn new(id: T::Key) -> Self {
        GenericFeeder { id, data: None }
    }
}

impl<T: CacheEntity> CacheFeed<T> for GenericFeeder<T> {
    fn entity_id(&mut self) -> T::Key {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<T>) {
        self.data = entity;
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
    fn test_generic_feeder() {
        let mut feeder = GenericFeeder::new("test_id".to_string());

        assert_eq!(feeder.entity_id(), "test_id");

        let entity = TestEntity {
            id: "test_id".to_string(),
            value: "data".to_string(),
        };

        feeder.feed(Some(entity.clone()));
        assert!(feeder.data.is_some());
    }

    #[test]
    fn test_feeder_validation() {
        let feeder: GenericFeeder<TestEntity> = GenericFeeder::new("id".to_string());
        assert!(feeder.validate().is_ok());
    }

    #[test]
    fn test_feeder_on_loaded_hook() {
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct TrackingFeeder {
            id: String,
            data: Option<TestEntity>,
            loaded_count: Arc<Mutex<usize>>,
        }

        impl CacheFeed<TestEntity> for TrackingFeeder {
            fn entity_id(&mut self) -> String {
                self.id.clone()
            }

            fn feed(&mut self, entity: Option<TestEntity>) {
                self.data = entity;
            }

            fn on_loaded(&mut self, _entity: &TestEntity) -> Result<()> {
                *self.loaded_count.lock().unwrap() += 1;
                Ok(())
            }
        }

        let loaded_count = Arc::new(Mutex::new(0));
        let mut feeder = TrackingFeeder {
            id: "1".to_string(),
            data: None,
            loaded_count: loaded_count.clone(),
        };

        let entity = TestEntity {
            id: "1".to_string(),
            value: "test".to_string(),
        };

        feeder.on_loaded(&entity).unwrap();
        assert_eq!(*loaded_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_feeder_on_hit_hook() {
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct HitTrackingFeeder {
            id: String,
            data: Option<TestEntity>,
            hit_keys: Arc<Mutex<Vec<String>>>,
        }

        impl CacheFeed<TestEntity> for HitTrackingFeeder {
            fn entity_id(&mut self) -> String {
                self.id.clone()
            }

            fn feed(&mut self, entity: Option<TestEntity>) {
                self.data = entity;
            }

            fn on_hit(&mut self, key: &str) -> Result<()> {
                self.hit_keys.lock().unwrap().push(key.to_string());
                Ok(())
            }
        }

        let hit_keys = Arc::new(Mutex::new(Vec::new()));
        let mut feeder = HitTrackingFeeder {
            id: "1".to_string(),
            data: None,
            hit_keys: hit_keys.clone(),
        };

        feeder.on_hit("test:1").unwrap();
        assert_eq!(hit_keys.lock().unwrap().len(), 1);
        assert_eq!(hit_keys.lock().unwrap()[0], "test:1");
    }

    #[test]
    fn test_feeder_on_miss_hook() {
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct MissTrackingFeeder {
            id: String,
            data: Option<TestEntity>,
            miss_keys: Arc<Mutex<Vec<String>>>,
        }

        impl CacheFeed<TestEntity> for MissTrackingFeeder {
            fn entity_id(&mut self) -> String {
                self.id.clone()
            }

            fn feed(&mut self, entity: Option<TestEntity>) {
                self.data = entity;
            }

            fn on_miss(&mut self, key: &str) -> Result<()> {
                self.miss_keys.lock().unwrap().push(key.to_string());
                Ok(())
            }
        }

        let miss_keys = Arc::new(Mutex::new(Vec::new()));
        let mut feeder = MissTrackingFeeder {
            id: "1".to_string(),
            data: None,
            miss_keys: miss_keys.clone(),
        };

        feeder.on_miss("test:1").unwrap();
        assert_eq!(miss_keys.lock().unwrap().len(), 1);
        assert_eq!(miss_keys.lock().unwrap()[0], "test:1");
    }

    #[test]
    fn test_generic_feeder_feed_none() {
        let mut feeder: GenericFeeder<TestEntity> = GenericFeeder::new("test_id".to_string());
        feeder.feed(None);
        assert!(feeder.data.is_none());
    }
}
