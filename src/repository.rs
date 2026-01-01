//! Data repository trait for abstracting database access.
//!
//! The `DataRepository` trait decouples cache-kit from specific database implementations.
//! This allows you to plug in any storage backend and makes testing your cache-using code
//! straightforward via mockable implementations.
//!
//! # Implementing DataRepository
//!
//! Implement this trait for any storage backend:
//! - SQL databases: SQLx, tokio-postgres, Diesel
//! - NoSQL: MongoDB, DynamoDB, Firestore
//! - In-memory: For testing (provided in this module)
//! - Custom ORMs or proprietary systems
//!
//! # Mocking for Tests
//!
//! Create simple in-memory implementations for unit testing:
//!
//! ```ignore
//! use cache_kit::repository::InMemoryRepository;
//! use cache_kit::entity::CacheEntity;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone, Serialize, Deserialize)]
//! pub struct User {
//!     id: String,
//!     name: String,
//! }
//!
//! impl CacheEntity for User {
//!     type Key = String;
//!     fn cache_key(&self) -> Self::Key { self.id.clone() }
//!     fn cache_prefix() -> &'static str { "user" }
//! }
//!
//! #[tokio::test]
//! async fn test_cache_with_mock_repo() {
//!     let mut repo = InMemoryRepository::new();
//!     repo.insert("user:1".to_string(), User {
//!         id: "user:1".to_string(),
//!         name: "Alice".to_string(),
//!     });
//!
//!     // Use repo in your cache-kit code
//!     let user = repo.fetch_by_id(&"user:1".to_string()).await.unwrap();
//!     assert_eq!(user.map(|u| u.name), Some("Alice".to_string()));
//! }
//! ```
//!
//! # Error Handling
//!
//! When implementing the trait for real databases, return `Err` for:
//! - Database connectivity issues
//! - Query timeouts
//! - Authentication failures
//! - Serialization errors
//! - Any other storage operation failures

use crate::entity::CacheEntity;
use crate::error::Result;

/// Trait for data repository implementations.
///
/// Abstracts database operations, decoupling cache from specific DB client.
/// Implementations: SQLx, tokio-postgres, Diesel, custom ORM, in-memory, etc.
///
/// # Design for Testability
///
/// This trait is designed to be mockable. Implement it with your database client,
/// or use `InMemoryRepository` provided in this module for testing.
#[allow(async_fn_in_trait)]
pub trait DataRepository<T: CacheEntity>: Send + Sync {
    /// Fetch entity by ID from primary data source.
    ///
    /// Called when cache miss occurs or on explicit refresh.
    ///
    /// # Returns
    /// - `Ok(Some(entity))` - Entity found
    /// - `Ok(None)` - Entity not found (not an error)
    /// - `Err(e)` - Database error
    ///
    /// # Errors
    /// Returns `Err` if data source is unavailable or fetch fails
    async fn fetch_by_id(&self, id: &T::Key) -> Result<Option<T>>;

    /// Batch fetch entities by IDs (optional optimization).
    ///
    /// Default implementation calls `fetch_by_id()` for each key.
    /// Override for efficiency (e.g., SQL `WHERE id IN (...)`)
    ///
    /// # Errors
    /// Returns `Err` if data source is unavailable or fetch fails
    async fn fetch_by_ids(&self, ids: &[T::Key]) -> Result<Vec<Option<T>>> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            results.push(self.fetch_by_id(id).await?);
        }
        Ok(results)
    }

    /// Count total entities (optional, for statistics).
    ///
    /// # Errors
    /// Returns `Err` if not implemented or if data source operation fails
    async fn count(&self) -> Result<u64> {
        Err(crate::error::Error::NotImplemented(
            "count not implemented".to_string(),
        ))
    }

    /// Optional: Get all entities (use sparingly, potentially large result).
    ///
    /// # Errors
    /// Returns `Err` if not implemented or if data source operation fails
    async fn fetch_all(&self) -> Result<Vec<T>> {
        Err(crate::error::Error::NotImplemented(
            "fetch_all not implemented for this repository".to_string(),
        ))
    }
}

// ============================================================================
// In-Memory Test Repository
// ============================================================================

use std::collections::HashMap;

/// Simple in-memory repository for testing cache-kit implementations.
///
/// Provides a straightforward mock `DataRepository` suitable for unit tests
/// where you want to control what data is "stored" without setting up a real database.
///
/// # Why Use InMemoryRepository
///
/// - **Fast Tests**: No database setup, teardown, or network calls
/// - **Deterministic**: Control exactly what data is present
/// - **Isolated**: Each test can have its own data without conflicts
/// - **Simple**: Easy to understand test behavior and debug failures
///
/// # Example Usage
///
/// ```ignore
/// #[tokio::test]
/// async fn test_cache_expander_with_mock_data() {
///     // Create and populate mock repository
///     let mut repo = InMemoryRepository::new();
///     repo.insert("user:1".to_string(), my_user_entity);
///
///     // Use with cache-kit components
///     let mut feeder = MyFeeder::new();
///     let result = expander.with(&mut feeder, &repo, CacheStrategy::Refresh).await;
///
///     assert!(result.is_ok());
/// }
/// ```
///
/// # Testing Different Scenarios
///
/// - **Cache hit**: Populate repo, cache will find the data
/// - **Cache miss**: Keep repo empty, cache will fallback to repo (which has nothing)
/// - **Invalidation**: Clear repo between operations to test refresh behavior
/// - **Batch operations**: Use `fetch_by_ids()` to test multi-key scenarios
pub struct InMemoryRepository<T: CacheEntity> {
    data: HashMap<String, T>,
}

impl<T: CacheEntity> InMemoryRepository<T> {
    /// Create a new empty in-memory repository.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let repo = InMemoryRepository::<MyEntity>::new();
    /// assert!(repo.is_empty());
    /// ```
    pub fn new() -> Self {
        InMemoryRepository {
            data: HashMap::new(),
        }
    }

    /// Insert or update an entity by key.
    ///
    /// # Example
    ///
    /// ```ignore
    /// repo.insert("user:123".to_string(), my_user);
    /// let found = repo.fetch_by_id(&"user:123".to_string()).await?;
    /// ```
    pub fn insert(&mut self, id: T::Key, value: T) {
        self.data.insert(id.to_string(), value);
    }

    /// Remove all entities from the repository.
    ///
    /// Useful for resetting state between test cases.
    ///
    /// # Example
    ///
    /// ```ignore
    /// repo.insert("user:1".to_string(), entity);
    /// assert_eq!(repo.len(), 1);
    /// repo.clear();
    /// assert!(repo.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Return the number of entities in the repository.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Return true if the repository contains no entities.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T: CacheEntity> Default for InMemoryRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: CacheEntity> DataRepository<T> for InMemoryRepository<T> {
    async fn fetch_by_id(&self, id: &T::Key) -> Result<Option<T>> {
        Ok(self.data.get(&id.to_string()).cloned())
    }

    async fn fetch_by_ids(&self, ids: &[T::Key]) -> Result<Vec<Option<T>>> {
        Ok(ids
            .iter()
            .map(|id| self.data.get(&id.to_string()).cloned())
            .collect())
    }

    async fn count(&self) -> Result<u64> {
        Ok(self.data.len() as u64)
    }

    async fn fetch_all(&self) -> Result<Vec<T>> {
        Ok(self.data.values().cloned().collect())
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

    #[tokio::test]
    async fn test_in_memory_repository() {
        let mut repo = InMemoryRepository::new();

        let entity = TestEntity {
            id: "1".to_string(),
            value: "data".to_string(),
        };

        repo.insert("1".to_string(), entity.clone());

        let fetched = repo
            .fetch_by_id(&"1".to_string())
            .await
            .expect("Failed to fetch");
        assert!(fetched.is_some());
        assert_eq!(fetched.expect("Entity not found").value, "data");
    }

    #[tokio::test]
    async fn test_in_memory_repository_miss() {
        let repo: InMemoryRepository<TestEntity> = InMemoryRepository::new();

        let fetched = repo
            .fetch_by_id(&"nonexistent".to_string())
            .await
            .expect("Failed to fetch");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_batch() {
        let mut repo = InMemoryRepository::new();

        repo.insert(
            "1".to_string(),
            TestEntity {
                id: "1".to_string(),
                value: "a".to_string(),
            },
        );

        repo.insert(
            "2".to_string(),
            TestEntity {
                id: "2".to_string(),
                value: "b".to_string(),
            },
        );

        let ids = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let results = repo
            .fetch_by_ids(&ids)
            .await
            .expect("Failed to fetch batch");

        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert!(results[1].is_some());
        assert!(results[2].is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_count() {
        let mut repo = InMemoryRepository::new();

        repo.insert(
            "1".to_string(),
            TestEntity {
                id: "1".to_string(),
                value: "a".to_string(),
            },
        );

        assert_eq!(repo.count().await.expect("Failed to count"), 1);
    }
}
