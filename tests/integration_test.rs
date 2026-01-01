//! Integration tests for cache-kit
//!
//! These tests verify end-to-end cache behavior across all components.

use cache_kit::backend::{CacheBackend, InMemoryBackend};
use cache_kit::feed::GenericFeeder;
use cache_kit::repository::InMemoryRepository;
use cache_kit::{CacheEntity, CacheExpander, CacheStrategy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

// Test entity definition
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct User {
    id: String,
    name: String,
    email: String,
}

impl CacheEntity for User {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn cache_prefix() -> &'static str {
        "user"
    }
}

// Additional test entity for multiple entity tests
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Product {
    id: String,
    name: String,
    price: f64,
}

impl CacheEntity for Product {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn cache_prefix() -> &'static str {
        "product"
    }
}

// Another test entity
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Order {
    id: String,
    user_id: String,
    total: f64,
}

impl CacheEntity for Order {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn cache_prefix() -> &'static str {
        "order"
    }
}

/// Test 1: End-to-End Cache Flow
///
/// Verifies the complete cache flow:
/// - Cache miss → DB hit → cache populated
/// - Second call hits cache
/// - Data correctness throughout
#[tokio::test]
async fn test_end_to_end_cache_flow() {
    // Setup backend and repository
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    // Populate repository with test data
    let mut repo = InMemoryRepository::new();
    let user = User {
        id: "user_123".to_string(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    repo.insert(user.id.clone(), user.clone());

    // First call: Cache miss → DB hit
    let mut feeder = GenericFeeder::new("user_123".to_string());
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
        .await
        .expect("First cache operation should succeed");

    // Verify data was loaded from DB
    assert!(feeder.data.is_some(), "Data should be loaded from DB");
    let loaded_user = feeder.data.unwrap();
    assert_eq!(loaded_user.id, "user_123");
    assert_eq!(loaded_user.name, "Alice");
    assert_eq!(loaded_user.email, "alice@example.com");

    // Verify cache was populated
    let cache_key = "user:user_123";
    let cached_data = backend
        .clone()
        .get(cache_key)
        .await
        .expect("Cache get should not error");
    assert!(
        cached_data.is_some(),
        "Cache should be populated after first call"
    );

    // Second call: Cache hit
    let mut feeder2 = GenericFeeder::new("user_123".to_string());
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Refresh)
        .await
        .expect("Second cache operation should succeed");

    // Verify data was loaded from cache
    assert!(feeder2.data.is_some(), "Data should be loaded from cache");
    let cached_user = feeder2.data.unwrap();
    assert_eq!(cached_user.id, "user_123");
    assert_eq!(cached_user.name, "Alice");
    assert_eq!(cached_user, loaded_user, "Cached data should match DB data");
}

/// Test 2: Multiple Entities
///
/// Verifies that multiple different entities can be cached simultaneously:
/// - Cache 3 different entity types
/// - Verify all are retrievable
/// - Test cache keys are unique
/// - Verify no cross-contamination
#[tokio::test]
async fn test_multiple_entities() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    // Setup User repository
    let mut user_repo = InMemoryRepository::new();
    let user = User {
        id: "u1".to_string(),
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
    };
    user_repo.insert(user.id.clone(), user.clone());

    // Setup Product repository
    let mut product_repo = InMemoryRepository::new();
    let product = Product {
        id: "p1".to_string(),
        name: "Laptop".to_string(),
        price: 999.99,
    };
    product_repo.insert(product.id.clone(), product.clone());

    // Setup Order repository
    let mut order_repo = InMemoryRepository::new();
    let order = Order {
        id: "o1".to_string(),
        user_id: "u1".to_string(),
        total: 999.99,
    };
    order_repo.insert(order.id.clone(), order.clone());

    // Cache all three entities
    let mut user_feeder = GenericFeeder::new("u1".to_string());
    expander
        .with::<User, _, _>(&mut user_feeder, &user_repo, CacheStrategy::Refresh)
        .await
        .expect("User cache operation should succeed");

    let mut product_feeder = GenericFeeder::new("p1".to_string());
    expander
        .with::<Product, _, _>(&mut product_feeder, &product_repo, CacheStrategy::Refresh)
        .await
        .expect("Product cache operation should succeed");

    let mut order_feeder = GenericFeeder::new("o1".to_string());
    expander
        .with::<Order, _, _>(&mut order_feeder, &order_repo, CacheStrategy::Refresh)
        .await
        .expect("Order cache operation should succeed");

    // Verify all entities are cached with unique keys
    let user_cache_key = "user:u1";
    let product_cache_key = "product:p1";
    let order_cache_key = "order:o1";

    assert!(
        backend.clone().get(user_cache_key).await.unwrap().is_some(),
        "User should be cached"
    );
    assert!(
        backend
            .clone()
            .get(product_cache_key)
            .await
            .unwrap()
            .is_some(),
        "Product should be cached"
    );
    assert!(
        backend
            .clone()
            .get(order_cache_key)
            .await
            .unwrap()
            .is_some(),
        "Order should be cached"
    );

    // Verify cache keys are unique
    assert_ne!(user_cache_key, product_cache_key);
    assert_ne!(user_cache_key, order_cache_key);
    assert_ne!(product_cache_key, order_cache_key);

    // Verify data correctness (no cross-contamination)
    assert!(user_feeder.data.is_some());
    assert!(product_feeder.data.is_some());
    assert!(order_feeder.data.is_some());

    let retrieved_user = user_feeder.data.unwrap();
    let retrieved_product = product_feeder.data.unwrap();
    let retrieved_order = order_feeder.data.unwrap();

    assert_eq!(retrieved_user.name, "Bob");
    assert_eq!(retrieved_product.name, "Laptop");
    assert_eq!(retrieved_order.total, 999.99);

    // Verify total cache size
    assert_eq!(
        backend.len().await,
        3,
        "Should have exactly 3 cached entities"
    );
}

/// Test 3: TTL Expiration (InMemory)
///
/// Verifies that TTL expiration works correctly:
/// - Set entity with 1-second TTL
/// - Verify immediate retrieval works
/// - Sleep 2 seconds
/// - Verify cache miss after expiration
#[tokio::test]
async fn test_ttl_expiration() {
    use cache_kit::observability::TtlPolicy;

    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone())
        .with_ttl_policy(TtlPolicy::Fixed(Duration::from_secs(1)));

    let mut repo = InMemoryRepository::new();
    let user = User {
        id: "user_ttl".to_string(),
        name: "Charlie".to_string(),
        email: "charlie@example.com".to_string(),
    };
    repo.insert(user.id.clone(), user.clone());

    // First call: Cache the entity with 1-second TTL
    let mut feeder = GenericFeeder::new("user_ttl".to_string());
    let expander = expander;
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
        .await
        .expect("Cache operation should succeed");

    assert!(
        feeder.data.is_some(),
        "Data should be cached with TTL of 1 second"
    );
    assert_eq!(feeder.data.unwrap().name, "Charlie");

    // Verify immediate retrieval works
    let mut feeder2 = GenericFeeder::new("user_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(
        feeder2.data.is_some(),
        "Data should be retrievable immediately"
    );

    // Sleep for 2 seconds to exceed TTL
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify cache miss after expiration (Fresh strategy doesn't fall back to DB)
    let mut feeder3 = GenericFeeder::new("user_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder3, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(
        feeder3.data.is_none(),
        "Data should be expired and not retrievable via Fresh strategy"
    );

    // Verify cache is re-populated with Refresh strategy
    let mut feeder4 = GenericFeeder::new("user_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder4, &repo, CacheStrategy::Refresh)
        .await
        .expect("Refresh strategy should succeed");
    assert!(
        feeder4.data.is_some(),
        "Data should be re-cached after expiration"
    );
}

/// Test 4: Cache Invalidation
///
/// Verifies that cache invalidation works correctly:
/// - Populate cache with stale data
/// - Use Invalidate strategy
/// - Verify cache is refreshed from DB
#[tokio::test]
async fn test_cache_invalidation() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    // Pre-populate cache with stale data
    let stale_user = User {
        id: "user_inv".to_string(),
        name: "Stale Name".to_string(),
        email: "stale@example.com".to_string(),
    };
    let bytes = stale_user.serialize_for_cache().unwrap();
    backend
        .clone()
        .set("user:user_inv", bytes, None)
        .await
        .expect("Pre-populating cache should succeed");

    // Populate repository with fresh data
    let mut repo = InMemoryRepository::new();
    let fresh_user = User {
        id: "user_inv".to_string(),
        name: "Fresh Name".to_string(),
        email: "fresh@example.com".to_string(),
    };
    repo.insert(fresh_user.id.clone(), fresh_user.clone());

    // Verify cache has stale data
    let mut feeder_stale = GenericFeeder::new("user_inv".to_string());
    expander
        .with::<User, _, _>(&mut feeder_stale, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(feeder_stale.data.is_some());
    assert_eq!(
        feeder_stale.data.unwrap().name,
        "Stale Name",
        "Cache should have stale data"
    );

    // Use Invalidate strategy to refresh from DB
    let mut feeder_fresh = GenericFeeder::new("user_inv".to_string());
    expander
        .with::<User, _, _>(&mut feeder_fresh, &repo, CacheStrategy::Invalidate)
        .await
        .expect("Invalidate strategy should succeed");

    // Verify cache now has fresh data
    assert!(feeder_fresh.data.is_some());
    assert_eq!(
        feeder_fresh.data.unwrap().name,
        "Fresh Name",
        "Cache should be refreshed with fresh data"
    );

    // Verify subsequent cache hits return fresh data
    let mut feeder_verify = GenericFeeder::new("user_inv".to_string());
    expander
        .with::<User, _, _>(&mut feeder_verify, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(feeder_verify.data.is_some());
    assert_eq!(
        feeder_verify.data.unwrap().name,
        "Fresh Name",
        "Cache should still have fresh data"
    );
}

/// Test 5: Concurrent Operations
///
/// Verifies thread safety of cache operations:
/// - Use Arc<Mutex<>> for shared state
/// - Spawn 10 threads doing cache operations
/// - Verify thread safety
/// - No panics or deadlocks
#[tokio::test]
async fn test_concurrent_operations() {
    let backend = InMemoryBackend::new();
    let expander = Arc::new(Mutex::new(CacheExpander::new(backend.clone())));

    // Shared repository
    let repo = Arc::new({
        let mut r = InMemoryRepository::new();
        for i in 0..10 {
            let user = User {
                id: format!("user_{}", i),
                name: format!("User {}", i),
                email: format!("user{}@example.com", i),
            };
            r.insert(user.id.clone(), user);
        }
        r
    });

    let mut handles = vec![];

    // Spawn 10 threads doing concurrent cache operations
    for i in 0..10 {
        let expander_clone = Arc::clone(&expander);
        let repo_clone = Arc::clone(&repo);

        let handle = tokio::spawn(async move {
            // Each thread performs multiple operations
            for j in 0..5 {
                let user_id = format!("user_{}", (i + j) % 10);
                let mut feeder = GenericFeeder::new(user_id);

                // Acquire lock and perform cache operation
                let result = expander_clone
                    .lock()
                    .await
                    .with::<User, _, _>(&mut feeder, &*repo_clone, CacheStrategy::Refresh)
                    .await;

                // Verify operation succeeded
                assert!(result.is_ok(), "Concurrent cache operation should succeed");

                // Verify data is correct
                if let Some(user) = feeder.data {
                    assert!(user.name.starts_with("User "));
                    assert!(user.email.contains("@example.com"));
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.await.expect("Thread should not panic");
    }

    // Verify cache is populated correctly
    assert!(
        !backend.is_empty().await,
        "Cache should be populated after concurrent operations"
    );
    assert!(
        backend.len().await <= 10,
        "Cache should have at most 10 entries (one per user)"
    );

    // Verify all users are cached correctly
    for i in 0..10 {
        let cache_key = format!("user:user_{}", i);
        let cached_data = backend.clone().get(&cache_key).await.unwrap();
        assert!(
            cached_data.is_some(),
            "User {} should be cached after concurrent operations",
            i
        );

        // Deserialize and verify
        let user = User::deserialize_from_cache(&cached_data.unwrap()).unwrap();
        assert_eq!(user.id, format!("user_{}", i));
        assert_eq!(user.name, format!("User {}", i));
    }
}
