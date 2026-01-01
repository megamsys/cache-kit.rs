//! High-level cache service for web applications.
//!
//! Provides a convenient wrapper around CacheExpander with Arc for easy sharing.

use crate::backend::CacheBackend;
use crate::entity::CacheEntity;
use crate::error::Result;
use crate::expander::{CacheExpander, OperationConfig};
use crate::feed::CacheFeed;
use crate::observability::CacheMetrics;
use crate::repository::DataRepository;
use crate::strategy::CacheStrategy;
use std::str::FromStr;
use std::sync::Arc;

/// High-level cache service for web applications.
///
/// Wraps `CacheExpander` in `Arc` for easy sharing across threads without
/// requiring external `Arc<Mutex<>>` wrappers.
///
/// # Design
///
/// Since `CacheBackend` implementations use interior mutability (RwLock, Mutex),
/// and `CacheExpander` now uses `&self` methods, we can safely wrap it in `Arc`
/// without needing an additional `Mutex`.
///
/// # Example
///
/// ```ignore
/// use cache_kit::{CacheService, backend::InMemoryBackend};
///
/// // Create service (can be shared across threads)
/// let cache = CacheService::new(InMemoryBackend::new());
///
/// // In your web service struct
/// pub struct UserService {
///     cache: CacheService<InMemoryBackend>,
///     repo: Arc<UserRepository>,
/// }
///
/// impl UserService {
///     pub fn get(&self, id: &str) -> Result<Option<User>> {
///         let mut feeder = UserFeeder { id: id.to_string(), user: None };
///         self.cache.execute(&mut feeder, &*self.repo, CacheStrategy::Refresh)?;
///         Ok(feeder.user)
///     }
/// }
/// ```
#[derive(Clone)]
pub struct CacheService<B: CacheBackend> {
    expander: Arc<CacheExpander<B>>,
}

impl<B: CacheBackend> CacheService<B> {
    /// Create a new cache service with the given backend.
    pub fn new(backend: B) -> Self {
        CacheService {
            expander: Arc::new(CacheExpander::new(backend)),
        }
    }

    /// Create a new cache service with custom metrics.
    pub fn with_metrics(backend: B, metrics: Box<dyn CacheMetrics>) -> Self {
        CacheService {
            expander: Arc::new(CacheExpander::new(backend).with_metrics(metrics)),
        }
    }

    /// Execute a cache operation.
    ///
    /// This is equivalent to calling `expander.with()` but more ergonomic
    /// for service-oriented architectures.
    ///
    /// # Arguments
    ///
    /// - `feeder`: Entity feeder (implements `CacheFeed<T>`)
    /// - `repository`: Data repository (implements `DataRepository<T>`)
    /// - `strategy`: Cache strategy (Fresh, Refresh, Invalidate, Bypass)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cache = CacheService::new(InMemoryBackend::new());
    /// let mut feeder = UserFeeder { id: "user_123".to_string(), user: None };
    /// let repo = UserRepository::new(pool);
    ///
    /// cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await?;
    /// let user = feeder.user;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` in these cases:
    /// - `Error::ValidationError`: Feeder validation fails
    /// - `Error::DeserializationError`: Cached data is corrupted
    /// - `Error::InvalidCacheEntry`: Invalid cache envelope
    /// - `Error::VersionMismatch`: Schema version mismatch
    /// - `Error::BackendError`: Cache backend unavailable
    /// - `Error::RepositoryError`: Database access fails
    /// - `Error::SerializationError`: Entity serialization fails
    pub async fn execute<T, F, R>(
        &self,
        feeder: &mut F,
        repository: &R,
        strategy: CacheStrategy,
    ) -> Result<()>
    where
        T: CacheEntity,
        F: CacheFeed<T>,
        R: DataRepository<T>,
        T::Key: FromStr,
    {
        self.expander
            .with::<T, F, R>(feeder, repository, strategy)
            .await
    }

    /// Execute a cache operation with advanced configuration.
    ///
    /// This method allows per-operation configuration such as TTL override and retry logic,
    /// while working seamlessly with Arc-wrapped services.
    ///
    /// # Arguments
    ///
    /// - `feeder`: Entity feeder (implements `CacheFeed<T>`)
    /// - `repository`: Data repository (implements `DataRepository<T>`)
    /// - `strategy`: Cache strategy (Fresh, Refresh, Invalidate, Bypass)
    /// - `config`: Operation configuration (TTL override, retry count)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cache = CacheService::new(InMemoryBackend::new());
    /// let mut feeder = UserFeeder { id: "user_123".to_string(), user: None };
    /// let repo = UserRepository::new(pool);
    ///
    /// let config = OperationConfig::default()
    ///     .with_ttl(Duration::from_secs(300))
    ///     .with_retry(3);
    ///
    /// cache.execute_with_config(&mut feeder, &repo, CacheStrategy::Refresh, config).await?;
    /// let user = feeder.user;
    /// ```
    ///
    /// # Errors
    ///
    /// Same error cases as `execute()`, plus retry-related failures.
    pub async fn execute_with_config<T, F, R>(
        &self,
        feeder: &mut F,
        repository: &R,
        strategy: CacheStrategy,
        config: OperationConfig,
    ) -> Result<()>
    where
        T: CacheEntity,
        F: CacheFeed<T>,
        R: DataRepository<T>,
        T::Key: FromStr,
    {
        self.expander
            .with_config::<T, F, R>(feeder, repository, strategy, config)
            .await
    }

    /// Get a reference to the underlying expander.
    ///
    /// Use this if you need direct access to expander methods.
    pub fn expander(&self) -> &CacheExpander<B> {
        &self.expander
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InMemoryBackend;
    use crate::feed::GenericFeeder;
    use crate::repository::InMemoryRepository;
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
    fn test_cache_service_creation() {
        let backend = InMemoryBackend::new();
        let _service = CacheService::new(backend);
    }

    #[tokio::test]
    async fn test_cache_service_execute() {
        let backend = InMemoryBackend::new();
        let service = CacheService::new(backend);

        let mut repo = InMemoryRepository::new();
        repo.insert(
            "1".to_string(),
            TestEntity {
                id: "1".to_string(),
                value: "test_value".to_string(),
            },
        );

        let mut feeder = GenericFeeder::new("1".to_string());

        service
            .execute::<TestEntity, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
            .await
            .expect("Failed to execute");

        assert!(feeder.data.is_some());
        assert_eq!(feeder.data.expect("Data not found").value, "test_value");
    }

    #[test]
    fn test_cache_service_clone() {
        let backend = InMemoryBackend::new();
        let service1 = CacheService::new(backend);
        let service2 = service1.clone();

        // Both services share the same expander
        assert!(Arc::ptr_eq(&service1.expander, &service2.expander));
    }

    #[test]
    fn test_cache_service_expander_access() {
        let backend = InMemoryBackend::new();
        let service = CacheService::new(backend);

        // Can access expander directly
        let _expander = service.expander();
    }

    #[tokio::test]
    async fn test_cache_service_thread_safety() {
        let backend = InMemoryBackend::new();
        let service = CacheService::new(backend);

        let mut handles = vec![];

        for i in 0..5 {
            let service_clone = service.clone();
            let handle = tokio::spawn(async move {
                let mut repo = InMemoryRepository::new();
                repo.insert(
                    format!("{}", i),
                    TestEntity {
                        id: format!("{}", i),
                        value: format!("value_{}", i),
                    },
                );

                let mut feeder = GenericFeeder::new(format!("{}", i));
                service_clone
                    .execute::<TestEntity, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
                    .await
                    .expect("Failed to execute");

                assert!(feeder.data.is_some());
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task failed");
        }
    }

    #[tokio::test]
    async fn test_cache_service_execute_with_config() {
        use std::time::Duration;

        let backend = InMemoryBackend::new();
        let service = CacheService::new(backend);

        let mut repo = InMemoryRepository::new();
        repo.insert(
            "1".to_string(),
            TestEntity {
                id: "1".to_string(),
                value: "test_value".to_string(),
            },
        );

        let mut feeder = GenericFeeder::new("1".to_string());

        // Test with custom config
        let config = OperationConfig::default()
            .with_ttl(Duration::from_secs(300))
            .with_retry(3);

        service
            .execute_with_config::<TestEntity, _, _>(
                &mut feeder,
                &repo,
                CacheStrategy::Refresh,
                config,
            )
            .await
            .expect("Failed to execute with config");

        assert!(feeder.data.is_some());
        assert_eq!(feeder.data.expect("Data not found").value, "test_value");
    }
}
