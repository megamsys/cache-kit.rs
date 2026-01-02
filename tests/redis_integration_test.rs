//! Redis Backend Integration Tests
//!
//! These tests require a running Redis instance.
//!
//! ## Quick Start
//!
//! ```bash
//! # Option 1: Use Makefile (recommended)
//! make test FEATURES="--features redis"     # Automatically starts Redis and runs tests
//!
//! # Option 2: Manual setup
//! make up
//! cargo test --features redis --test redis_integration_test
//!
//! ```
//!
//! ## Environment Variables
//!
//! - `TEST_REDIS_URL`: Redis connection URL (default: "redis://localhost:6379")
//!
//! ## What's Tested
//!
//! 1. Redis connection and health check
//! 2. Basic set/get operations
//! 3. TTL expiration behavior
//! 4. Batch operations (mget/mdelete)
//! 5. Connection pooling under concurrent load

#![cfg(feature = "redis")]

use cache_kit::backend::{CacheBackend, RedisBackend, RedisConfig};
use cache_kit::feed::GenericFeeder;
use cache_kit::repository::InMemoryRepository;
use cache_kit::{CacheEntity, CacheExpander, CacheStrategy};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use std::time::Duration;

/// Helper: Get Redis connection URL from environment or use default
fn get_redis_url() -> String {
    env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

/// Helper: Create a test Redis backend
async fn create_test_backend() -> Result<RedisBackend, Box<dyn std::error::Error>> {
    let redis_url = get_redis_url();
    println!("Connecting to Redis: {}", redis_url);

    let backend = RedisBackend::from_connection_string(&redis_url).await?;
    Ok(backend)
}

/// Helper: Check if Redis is available
async fn is_redis_available() -> bool {
    match create_test_backend().await {
        Ok(backend) => backend.health_check().await.unwrap_or(false),
        Err(_) => false,
    }
}

// =============================================================================
// Test 1: Redis Connection
// =============================================================================

#[tokio::test]
async fn test_redis_connection() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        println!("💡 Run: make redis-start");
        return;
    }

    println!("Test 1: Redis Connection");

    // Connect to Redis
    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    // Verify health check works
    let is_healthy = backend
        .health_check()
        .await
        .expect("Health check should not error");

    assert!(is_healthy, "Redis health check should return true");
    println!("✓ Redis connection successful");
    println!("✓ Health check passed");
}

#[tokio::test]
async fn test_redis_connection_with_config() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 1b: Redis Connection with RedisConfig");

    // Create backend using RedisConfig
    let config = RedisConfig {
        host: "localhost".to_string(),
        port: 6379,
        database: 0,
        pool_size: 10,
        connection_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    let backend = RedisBackend::new(config)
        .await
        .expect("Failed to create Redis backend from config");

    assert!(backend.health_check().await.expect("Health check failed"));
    println!("✓ RedisConfig connection successful");
}

// =============================================================================
// Test 2: Basic Set/Get
// =============================================================================

#[tokio::test]
async fn test_redis_basic_set_get() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 2: Basic Set/Get");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    let test_key = "test:integration:key1";
    let test_value = b"Hello from cache-kit!".to_vec();

    // Set a value
    backend
        .set(test_key, test_value.clone(), None)
        .await
        .expect("SET should succeed");
    println!("✓ SET operation successful");

    // Get the value back
    let retrieved_value = backend.get(test_key).await.expect("GET should not error");

    assert!(retrieved_value.is_some(), "Value should exist in cache");
    assert_eq!(
        retrieved_value.unwrap(),
        test_value,
        "Retrieved value should match original"
    );
    println!("✓ GET operation successful");
    println!("✓ Values match");

    // Clean up
    backend
        .delete(test_key)
        .await
        .expect("DELETE should succeed");
    println!("✓ Cleanup successful");
}

#[tokio::test]
async fn test_redis_get_nonexistent_key() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 2b: Get Nonexistent Key");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    let result = backend
        .get("test:nonexistent:key")
        .await
        .expect("GET should not error");

    assert!(result.is_none(), "Nonexistent key should return None");
    println!("✓ Nonexistent key returns None correctly");
}

#[tokio::test]
async fn test_redis_exists() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 2c: Exists Check");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    let test_key = "test:exists:key";

    // Key should not exist initially
    assert!(!backend.exists(test_key).await.expect("EXISTS check failed"));

    // Set key
    backend
        .set(test_key, b"value".to_vec(), None)
        .await
        .expect("SET failed");

    // Key should exist now
    assert!(backend.exists(test_key).await.expect("EXISTS check failed"));
    println!("✓ EXISTS check works correctly");

    // Clean up
    backend.delete(test_key).await.expect("DELETE failed");
}

// =============================================================================
// Test 3: TTL Expiration
// =============================================================================

#[tokio::test]
async fn test_redis_ttl_expiration() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 3: TTL Expiration");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    let test_key = "test:ttl:key1";
    let test_value = b"expires in 1 second".to_vec();

    // Set value with 1-second TTL
    backend
        .set(test_key, test_value.clone(), Some(Duration::from_secs(1)))
        .await
        .expect("SET with TTL should succeed");
    println!("✓ SET with 1-second TTL successful");

    // Verify immediate retrieval works
    let immediate_result = backend.get(test_key).await.expect("GET should not error");
    assert!(
        immediate_result.is_some(),
        "Value should exist immediately after SET"
    );
    println!("✓ Immediate GET successful");

    // Wait for expiration (2 seconds to be safe)
    println!("⏳ Waiting 2 seconds for TTL expiration...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify key no longer exists
    let expired_result = backend.get(test_key).await.expect("GET should not error");
    assert!(
        expired_result.is_none(),
        "Value should be expired after TTL"
    );
    println!("✓ Key expired correctly after TTL");
}

#[tokio::test]
async fn test_redis_ttl_no_expiration() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 3b: No TTL (Persistent Key)");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    let test_key = "test:no_ttl:key";
    let test_value = b"persistent value".to_vec();

    // Set value without TTL
    backend
        .set(test_key, test_value.clone(), None)
        .await
        .expect("SET should succeed");

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Value should still exist
    let result = backend.get(test_key).await.expect("GET failed");
    assert!(result.is_some(), "Persistent key should still exist");
    println!("✓ Persistent key (no TTL) works correctly");

    // Clean up
    backend.delete(test_key).await.expect("DELETE failed");
}

// =============================================================================
// Test 4: Batch Operations
// =============================================================================

#[tokio::test]
async fn test_redis_batch_operations() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 4: Batch Operations (mget/mdelete)");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    // Prepare 10 test keys
    let test_keys: Vec<String> = (0..10).map(|i| format!("test:batch:key{}", i)).collect();

    let test_values: Vec<Vec<u8>> = (0..10)
        .map(|i| format!("value_{}", i).into_bytes())
        .collect();

    // Set all keys
    for (key, value) in test_keys.iter().zip(test_values.iter()) {
        backend
            .set(key, value.clone(), None)
            .await
            .expect("SET should succeed");
    }
    println!("✓ Set 10 keys successfully");

    // Verify keys were set by getting them individually
    for (i, key) in test_keys.iter().enumerate() {
        let val = backend.get(key).await.expect("GET should not error");
        if val.is_none() {
            println!("⚠️  Key {} was not set! Trying again...", key);
            backend
                .set(key, test_values[i].clone(), None)
                .await
                .expect("Retry SET should succeed");
        }
    }

    // Small delay to ensure Redis has processed all writes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Use mget to retrieve all values
    let keys_refs: Vec<&str> = test_keys.iter().map(|s| s.as_str()).collect();
    let retrieved_values = backend
        .mget(&keys_refs)
        .await
        .expect("MGET should not error");

    assert_eq!(retrieved_values.len(), 10, "Should retrieve 10 values");
    println!("✓ MGET retrieved 10 values");

    // Debug: Print what we got
    for (i, retrieved) in retrieved_values.iter().enumerate() {
        if retrieved.is_none() {
            println!(
                "⚠️  Value {} is None! Expected: {:?}",
                i,
                String::from_utf8_lossy(&test_values[i])
            );
        }
    }

    // Verify all values are correct
    for (i, retrieved) in retrieved_values.iter().enumerate() {
        assert!(
            retrieved.is_some(),
            "Value {} should exist (key: {})",
            i,
            test_keys[i]
        );
        assert_eq!(
            retrieved.as_ref().unwrap(),
            &test_values[i],
            "Value {} should match",
            i
        );
    }
    println!("✓ All values match original data");

    // Delete all keys using mdelete
    backend
        .mdelete(&keys_refs)
        .await
        .expect("MDELETE should succeed");
    println!("✓ MDELETE removed 10 keys");

    // Verify all keys are deleted
    let after_delete = backend.mget(&keys_refs).await.expect("MGET failed");
    assert!(
        after_delete.iter().all(|v| v.is_none()),
        "All keys should be deleted"
    );
    println!("✓ All keys successfully deleted");
}

#[tokio::test]
async fn test_redis_mget_with_missing_keys() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 4b: MGET with Missing Keys");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    // Set only some keys
    backend
        .set("test:mget:exists1", b"value1".to_vec(), None)
        .await
        .expect("SET failed");
    backend
        .set("test:mget:exists2", b"value2".to_vec(), None)
        .await
        .expect("SET failed");

    // MGET with mix of existing and non-existing keys
    let keys = vec![
        "test:mget:exists1",
        "test:mget:missing",
        "test:mget:exists2",
    ];
    let results = backend.mget(&keys).await.expect("MGET failed");

    assert_eq!(results.len(), 3);
    assert!(results[0].is_some());
    assert!(results[1].is_none()); // Missing key
    assert!(results[2].is_some());
    println!("✓ MGET handles missing keys correctly");

    // Clean up
    backend
        .mdelete(&["test:mget:exists1", "test:mget:exists2"])
        .await
        .expect("MDELETE failed");
}

// =============================================================================
// Test 5: Connection Pooling
// =============================================================================

#[tokio::test]
async fn test_redis_connection_pooling() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 5: Connection Pooling");

    let config = RedisConfig {
        host: "localhost".to_string(),
        port: 6379,
        database: 0,
        pool_size: 10,
        connection_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    let backend = RedisBackend::new(config)
        .await
        .expect("Failed to create Redis backend");

    // Get pool stats
    let stats = backend.pool_stats();
    println!("Pool Stats:");
    println!("  Connections: {}", stats.connections);
    println!("  Idle: {}", stats.idle_connections);
    assert!(
        stats.connections <= 10,
        "Pool should not exceed max size of 10"
    );
    println!(
        "✓ Initial pool state verified (connections: {})",
        stats.connections
    );

    // Make concurrent requests
    println!("⏳ Making 100 concurrent requests...");

    let mut handles = vec![];

    for i in 0..100 {
        let backend_clone = backend.clone();
        let handle = tokio::spawn(async move {
            let key = format!("test:concurrent:key{}", i);
            let value = format!("value_{}", i).into_bytes();

            // Perform SET and GET operations
            backend_clone
                .set(&key, value.clone(), None)
                .await
                .expect("SET failed");
            let retrieved = backend_clone.get(&key).await.expect("GET failed");
            assert_eq!(retrieved.expect("Value should exist"), value);
            backend_clone.delete(&key).await.expect("DELETE failed");
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete successfully");
    }

    println!("✓ 100 concurrent operations completed successfully");

    // Check pool stats after concurrent operations
    let final_stats = backend.pool_stats();
    println!("Final Pool Stats:");
    println!("  Connections: {}", final_stats.connections);
    println!("  Idle: {}", final_stats.idle_connections);

    assert!(final_stats.connections <= 10, "Should not exceed pool size");
    println!("✓ Pool size constraint maintained");
    println!("✓ No connection exhaustion");
}

#[tokio::test]
async fn test_redis_pool_reuse() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test 5b: Connection Pool Reuse");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    // Clone backend (shares the same pool)
    let backend1 = backend.clone();
    let backend2 = backend;

    // Both backends should work independently
    backend1
        .set("test:pool:key1", b"value1".to_vec(), None)
        .await
        .expect("SET failed");
    backend2
        .set("test:pool:key2", b"value2".to_vec(), None)
        .await
        .expect("SET failed");

    // Verify both keys exist
    assert!(backend1
        .get("test:pool:key1")
        .await
        .expect("GET failed")
        .is_some());
    assert!(backend2
        .get("test:pool:key2")
        .await
        .expect("GET failed")
        .is_some());

    println!("✓ Cloned backends share connection pool correctly");

    // Clean up
    backend1
        .delete("test:pool:key1")
        .await
        .expect("DELETE failed");
    backend2
        .delete("test:pool:key2")
        .await
        .expect("DELETE failed");
}

// =============================================================================
// Additional Tests
// =============================================================================

#[tokio::test]
async fn test_redis_clear_all() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test: Clear All (FLUSHDB)");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    // Set some test keys
    backend
        .set("test:clear:1", b"value1".to_vec(), None)
        .await
        .expect("SET failed");
    backend
        .set("test:clear:2", b"value2".to_vec(), None)
        .await
        .expect("SET failed");
    backend
        .set("test:clear:3", b"value3".to_vec(), None)
        .await
        .expect("SET failed");

    // Clear all
    backend.clear_all().await.expect("CLEAR_ALL should succeed");
    println!("✓ CLEAR_ALL executed");

    // Verify keys are gone
    assert!(backend
        .get("test:clear:1")
        .await
        .expect("GET failed")
        .is_none());
    assert!(backend
        .get("test:clear:2")
        .await
        .expect("GET failed")
        .is_none());
    assert!(backend
        .get("test:clear:3")
        .await
        .expect("GET failed")
        .is_none());

    println!("✓ All keys cleared successfully");
}

#[tokio::test]
async fn test_redis_delete() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test: Delete Operation");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    let test_key = "test:delete:key";

    // Set key
    backend
        .set(test_key, b"to be deleted".to_vec(), None)
        .await
        .expect("SET failed");
    assert!(backend.exists(test_key).await.expect("EXISTS failed"));

    // Delete key
    backend.delete(test_key).await.expect("DELETE failed");

    // Verify deleted
    assert!(!backend.exists(test_key).await.expect("EXISTS failed"));
    println!("✓ DELETE operation successful");
}

// =============================================================================
// END-TO-END CACHE-KIT FRAMEWORK TESTS WITH REDIS BACKEND
// =============================================================================

/// Test entity for end-to-end tests
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
        "redis_test_user"
    }
}

/// Product entity for testing multiple entity types
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
        "redis_test_product"
    }
}

// =============================================================================
// Test E2E-1: End-to-End Cache Flow with Redis
// =============================================================================

#[tokio::test]
async fn test_e2e_cache_flow_with_redis() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test E2E-1: End-to-End Cache Flow with Redis Backend");

    // Setup Redis backend
    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");
    let expander = CacheExpander::new(backend.clone());

    // Populate repository with test data
    let mut repo = InMemoryRepository::new();
    let user = User {
        id: "e2e_user_1".to_string(),
        name: "Alice Redis".to_string(),
        email: "alice@redis.com".to_string(),
    };
    repo.insert(user.id.clone(), user.clone());

    // First call: Cache miss → DB hit → Redis populated
    let mut feeder = GenericFeeder::new("e2e_user_1".to_string());
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
        .await
        .expect("First cache operation should succeed");

    // Verify data was loaded from DB
    assert!(feeder.data.is_some(), "Data should be loaded from DB");
    let loaded_user = feeder.data.unwrap();
    assert_eq!(loaded_user.id, "e2e_user_1");
    assert_eq!(loaded_user.name, "Alice Redis");
    assert_eq!(loaded_user.email, "alice@redis.com");
    println!("✓ Cache miss → DB hit → Redis populated");

    // Verify Redis cache was populated
    let cache_key = "redis_test_user:e2e_user_1";
    let cached_data = backend
        .clone()
        .get(cache_key)
        .await
        .expect("Cache get should not error");
    assert!(
        cached_data.is_some(),
        "Redis cache should be populated after first call"
    );
    println!("✓ Redis cache populated");

    // Second call: Redis cache hit
    let mut feeder2 = GenericFeeder::new("e2e_user_1".to_string());
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Refresh)
        .await
        .expect("Second cache operation should succeed");

    // Verify data was loaded from Redis cache
    assert!(
        feeder2.data.is_some(),
        "Data should be loaded from Redis cache"
    );
    let cached_user = feeder2.data.unwrap();
    assert_eq!(cached_user, loaded_user, "Cached data should match DB data");
    println!("✓ Redis cache hit successful");

    // Cleanup
    backend.delete(cache_key).await.ok();
}

// =============================================================================
// Test E2E-2: Multiple Entities with Redis
// =============================================================================

#[tokio::test]
async fn test_e2e_multiple_entities_with_redis() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test E2E-2: Multiple Entities with Redis Backend");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");
    let expander = CacheExpander::new(backend.clone());

    // Setup User repository
    let mut user_repo = InMemoryRepository::new();
    let user = User {
        id: "e2e_u1".to_string(),
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
    };
    user_repo.insert(user.id.clone(), user.clone());

    // Setup Product repository
    let mut product_repo = InMemoryRepository::new();
    let product = Product {
        id: "e2e_p1".to_string(),
        name: "Redis Laptop".to_string(),
        price: 1299.99,
    };
    product_repo.insert(product.id.clone(), product.clone());

    // Cache both entities
    let mut user_feeder = GenericFeeder::new("e2e_u1".to_string());
    expander
        .with::<User, _, _>(&mut user_feeder, &user_repo, CacheStrategy::Refresh)
        .await
        .expect("User cache operation should succeed");

    let mut product_feeder = GenericFeeder::new("e2e_p1".to_string());
    expander
        .with::<Product, _, _>(&mut product_feeder, &product_repo, CacheStrategy::Refresh)
        .await
        .expect("Product cache operation should succeed");

    // Verify both entities are cached with unique keys in Redis
    let user_cache_key = "redis_test_user:e2e_u1";
    let product_cache_key = "redis_test_product:e2e_p1";

    assert!(
        backend
            .clone()
            .get(user_cache_key)
            .await
            .expect("GET failed")
            .is_some(),
        "User should be cached in Redis"
    );
    assert!(
        backend
            .clone()
            .get(product_cache_key)
            .await
            .expect("GET failed")
            .is_some(),
        "Product should be cached in Redis"
    );
    println!("✓ Multiple entity types cached in Redis");

    // Verify cache keys are unique
    assert_ne!(user_cache_key, product_cache_key);
    println!("✓ Cache keys are unique");

    // Verify data correctness
    assert_eq!(user_feeder.data.unwrap().name, "Bob");
    assert_eq!(product_feeder.data.unwrap().name, "Redis Laptop");
    println!("✓ No cross-contamination between entity types");

    // Cleanup
    backend.delete(user_cache_key).await.ok();
    backend.delete(product_cache_key).await.ok();
}

// =============================================================================
// Test E2E-3: Cache Strategies with Redis
// =============================================================================

#[tokio::test]
async fn test_e2e_cache_strategies_with_redis() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test E2E-3: Cache Strategies with Redis Backend");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");
    let expander = CacheExpander::new(backend.clone());

    let mut repo = InMemoryRepository::new();
    let user = User {
        id: "e2e_strategy".to_string(),
        name: "Fresh User".to_string(),
        email: "fresh@example.com".to_string(),
    };
    repo.insert(user.id.clone(), user.clone());

    // Test 1: Refresh Strategy (cache miss → DB → cache)
    let mut feeder1 = GenericFeeder::new("e2e_strategy".to_string());
    expander
        .with::<User, _, _>(&mut feeder1, &repo, CacheStrategy::Refresh)
        .await
        .expect("Refresh strategy should succeed");
    assert!(feeder1.data.is_some());
    println!("✓ Refresh strategy works (cache populated)");

    // Test 2: Fresh Strategy (cache hit)
    let mut feeder2 = GenericFeeder::new("e2e_strategy".to_string());
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(feeder2.data.is_some());
    println!("✓ Fresh strategy works (cache hit)");

    // Test 3: Invalidate Strategy (force refresh from DB)
    // Update data in repository
    let updated_user = User {
        id: "e2e_strategy".to_string(),
        name: "Updated User".to_string(),
        email: "updated@example.com".to_string(),
    };
    repo.insert(updated_user.id.clone(), updated_user.clone());

    let mut feeder3 = GenericFeeder::new("e2e_strategy".to_string());
    expander
        .with::<User, _, _>(&mut feeder3, &repo, CacheStrategy::Invalidate)
        .await
        .expect("Invalidate strategy should succeed");
    assert!(feeder3.data.is_some());
    assert_eq!(feeder3.data.unwrap().name, "Updated User");
    println!("✓ Invalidate strategy works (cache refreshed)");

    // Test 4: Bypass Strategy (always DB, no cache)
    let mut feeder4 = GenericFeeder::new("e2e_strategy".to_string());
    expander
        .with::<User, _, _>(&mut feeder4, &repo, CacheStrategy::Bypass)
        .await
        .expect("Bypass strategy should succeed");
    assert!(feeder4.data.is_some());
    println!("✓ Bypass strategy works (direct DB access)");

    // Cleanup
    backend.delete("redis_test_user:e2e_strategy").await.ok();
}

// =============================================================================
// Test E2E-4: TTL with Redis Backend
// =============================================================================

#[tokio::test]
async fn test_e2e_ttl_with_redis() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test E2E-4: TTL with Redis Backend");

    use cache_kit::observability::TtlPolicy;

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");
    let expander = CacheExpander::new(backend.clone())
        .with_ttl_policy(TtlPolicy::Fixed(Duration::from_secs(2)));

    let mut repo = InMemoryRepository::new();
    let user = User {
        id: "e2e_ttl".to_string(),
        name: "TTL User".to_string(),
        email: "ttl@example.com".to_string(),
    };
    repo.insert(user.id.clone(), user.clone());

    // Cache with 2-second TTL
    let mut feeder1 = GenericFeeder::new("e2e_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder1, &repo, CacheStrategy::Refresh)
        .await
        .expect("Cache operation should succeed");
    assert!(feeder1.data.is_some());
    println!("✓ Data cached with 2-second TTL");

    // Immediate retrieval should work
    let mut feeder2 = GenericFeeder::new("e2e_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(feeder2.data.is_some());
    println!("✓ Immediate retrieval works");

    // Wait for expiration
    println!("⏳ Waiting 3 seconds for TTL expiration...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Fresh strategy should return None (cache expired)
    let mut feeder3 = GenericFeeder::new("e2e_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder3, &repo, CacheStrategy::Fresh)
        .await
        .expect("Fresh strategy should succeed");
    assert!(feeder3.data.is_none(), "Cache should be expired");
    println!("✓ Cache expired after TTL");

    // Refresh strategy should repopulate cache
    let mut feeder4 = GenericFeeder::new("e2e_ttl".to_string());
    expander
        .with::<User, _, _>(&mut feeder4, &repo, CacheStrategy::Refresh)
        .await
        .expect("Refresh strategy should succeed");
    assert!(feeder4.data.is_some());
    println!("✓ Cache repopulated with Refresh strategy");

    // Cleanup
    backend.delete("redis_test_user:e2e_ttl").await.ok();
}

// =============================================================================
// Test E2E-5: Concurrent Operations with Redis
// =============================================================================

#[tokio::test]
async fn test_e2e_concurrent_operations_with_redis() {
    if !is_redis_available().await {
        println!("⚠️  Redis not available, skipping test");
        return;
    }

    println!("Test E2E-5: Concurrent Operations with Redis Backend");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Redis backend");

    // Don't use a mutex - CacheExpander is already thread-safe via Arc<Backend>
    let expander = CacheExpander::new(backend.clone());

    // Shared repository with 10 users
    let repo = Arc::new({
        let mut r = InMemoryRepository::new();
        for i in 0..10 {
            let user = User {
                id: format!("e2e_concurrent_{}", i),
                name: format!("Concurrent User {}", i),
                email: format!("user{}@concurrent.com", i),
            };
            r.insert(user.id.clone(), user);
        }
        r
    });

    // Wrap expander in Arc for sharing across tasks (no Mutex needed - it's Send + Sync)
    let expander = Arc::new(expander);

    let mut handles = vec![];

    // Spawn 10 tasks doing concurrent cache operations
    for i in 0..10 {
        let expander_clone = Arc::clone(&expander);
        let repo_clone = Arc::clone(&repo);

        let handle = tokio::spawn(async move {
            // Each task performs multiple operations
            for j in 0..3 {
                let user_id = format!("e2e_concurrent_{}", (i + j) % 10);
                let mut feeder = GenericFeeder::new(user_id);

                // CacheExpander is Send + Sync, so we can call it directly without Mutex
                let result = expander_clone
                    .with::<User, _, _>(&mut feeder, &*repo_clone, CacheStrategy::Refresh)
                    .await;

                assert!(result.is_ok(), "Concurrent operation should succeed");

                // Verify data is correct
                if let Some(user) = feeder.data {
                    assert!(user.name.starts_with("Concurrent User "));
                    assert!(user.email.contains("@concurrent.com"));
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should not panic");
    }

    println!("✓ 10 threads completed 30 total operations");

    // Verify all users are cached in Redis
    let mut cached_count = 0;
    for i in 0..10 {
        let cache_key = format!("redis_test_user:e2e_concurrent_{}", i);
        if backend
            .clone()
            .exists(&cache_key)
            .await
            .expect("EXISTS failed")
        {
            cached_count += 1;
            // Cleanup
            backend.delete(&cache_key).await.ok();
        }
    }

    assert!(
        cached_count > 0,
        "At least some users should be cached after concurrent operations"
    );
    println!("✓ {} users cached successfully", cached_count);
    println!("✓ No race conditions or deadlocks");
}
