//! Memcached Backend Integration Tests
//!
//! These tests require a running Memcached instance.
//!
//! ## Quick Start
//!
//! ```bash
//! # Option 1: Use Makefile (recommended)
//! make up                  # Start Redis and Memcached services
//! make test FEATURES="--features memcached"     # Run Memcached integration tests
//!
//! # Option 2: Manual setup
//! make up
//! cargo test --features memcached --test memcached_integration_test
//!
//! ```
//!
//! **Note:** Tests use unique key prefixes per test to avoid conflicts when run in parallel.
//!
//! ## Environment Variables
//!
//! - `TEST_MEMCACHED_URL`: Memcached server address (default: "localhost:11211")
//!
//! ## What's Tested
//!
//! 1. Memcached connection and health check
//! 2. Basic set/get operations
//! 3. TTL expiration behavior
//! 4. Multi-get operations (mget)
//! 5. Flush all (clear_all)
//! 6. Delete operations
//! 7. Exists checks

#![cfg(feature = "memcached")]

use cache_kit::backend::{CacheBackend, MemcachedBackend, MemcachedConfig};
use cache_kit::feed::GenericFeeder;
use cache_kit::repository::InMemoryRepository;
use cache_kit::{CacheEntity, CacheExpander, CacheStrategy};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Helper: Get Memcached server address from environment or use default
fn get_memcached_url() -> String {
    env::var("TEST_MEMCACHED_URL").unwrap_or_else(|_| "localhost:11211".to_string())
}

/// Helper: Generate a unique test key prefix for test isolation
///
/// Uses UUID v7 for guaranteed uniqueness across all parallel tests.
/// Format uses only alphanumeric and hyphens to comply with memcached key restrictions.
fn unique_test_key(base: &str) -> String {
    use uuid::Uuid;

    let uuid = Uuid::now_v7();
    // Use simple format without hyphens and colons
    let clean_base = base.replace(':', "_").replace('-', "_");
    format!("test_{}_{}", uuid.simple(), clean_base)
}

/// Helper: Generate multiple unique test keys
fn unique_test_keys(base: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|i| unique_test_key(&format!("{}:{}", base, i)))
        .collect()
}

/// Helper: Create a test Memcached backend
async fn create_test_backend() -> Result<MemcachedBackend, Box<dyn std::error::Error>> {
    let memcached_url = get_memcached_url();
    println!("Connecting to Memcached: {}", memcached_url);

    let config = MemcachedConfig {
        servers: vec![memcached_url],
        connection_timeout: Duration::from_secs(5),
        pool_size: 32, // Increased for parallel test execution
    };
    let backend = MemcachedBackend::new(config).await?;
    Ok(backend)
}

/// Helper: Check if Memcached is available
async fn is_memcached_available() -> bool {
    match create_test_backend().await {
        Ok(backend) => backend.health_check().await.unwrap_or(false),
        Err(_) => false,
    }
}

/// Helper: Cleanup test keys (best effort - ignores errors)
async fn cleanup_keys(backend: &MemcachedBackend, keys: &[String]) {
    for key in keys {
        let _ = backend.delete(key).await;
    }
}

// =============================================================================
// Test 1: Memcached Connection
// =============================================================================

#[tokio::test]
async fn test_memcached_connection() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        println!("💡 Run: make up");
        return;
    }

    println!("Test 1: Memcached Connection");

    // Connect to Memcached
    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    // Verify health check works
    let is_healthy = backend
        .health_check()
        .await
        .expect("Health check should not error");

    assert!(is_healthy, "Memcached health check should return true");
    println!("✓ Memcached connection successful");
    println!("✓ Health check passed");
}

#[tokio::test]
async fn test_memcached_connection_with_config() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 1b: Memcached Connection with MemcachedConfig");

    // Create backend using MemcachedConfig
    let config = MemcachedConfig {
        servers: vec!["localhost:11211".to_string()],
        connection_timeout: Duration::from_secs(5),
        pool_size: 10,
    };

    let backend = MemcachedBackend::new(config)
        .await
        .expect("Failed to create Memcached backend from config");

    assert!(backend.health_check().await.unwrap());
    println!("✓ MemcachedConfig connection successful");
}

// =============================================================================
// Test 2: Basic Set/Get
// =============================================================================

#[tokio::test]
async fn test_memcached_basic_set_get() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 2: Basic Set/Get");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_key = unique_test_key("key1");
    let test_value = b"Hello from cache-kit!".to_vec();

    // Set a value
    backend
        .set(&test_key, test_value.clone(), None)
        .await
        .expect("SET should succeed");
    println!("✓ SET operation successful");

    // Get the value back
    let retrieved_value = backend.get(&test_key).await.expect("GET should not error");

    assert!(retrieved_value.is_some(), "Value should exist in cache");
    assert_eq!(
        retrieved_value.unwrap(),
        test_value,
        "Retrieved value should match original"
    );
    println!("✓ GET operation successful");
    println!("✓ Values match");

    // Clean up
    cleanup_keys(&backend, &[test_key]).await;
    println!("✓ Cleanup successful");
}

#[tokio::test]
async fn test_memcached_get_nonexistent_key() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 2b: Get Nonexistent Key");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_key = unique_test_key("nonexistent");
    let result = backend.get(&test_key).await.expect("GET should not error");

    assert!(result.is_none(), "Nonexistent key should return None");
    println!("✓ Nonexistent key returns None correctly");
}

#[tokio::test]
async fn test_memcached_exists() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 2c: Exists Check");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_key = unique_test_key("exists");

    // Key should not exist initially
    assert!(!backend.exists(&test_key).await.unwrap());

    // Set key
    backend
        .set(&test_key, b"value".to_vec(), None)
        .await
        .unwrap();

    // Key should exist now
    assert!(backend.exists(&test_key).await.unwrap());
    println!("✓ EXISTS check works correctly");

    // Clean up
    cleanup_keys(&backend, &[test_key]).await;
}

// =============================================================================
// Test 3: TTL Expiration
// =============================================================================

#[tokio::test]
async fn test_memcached_ttl_expiration() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 3: TTL Expiration");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_key = unique_test_key("ttl");
    let test_value = b"expires in 1 second".to_vec();

    // Set value with 1-second TTL
    backend
        .set(&test_key, test_value.clone(), Some(Duration::from_secs(1)))
        .await
        .expect("SET with TTL should succeed");
    println!("✓ SET with 1-second TTL successful");

    // Verify immediate retrieval works
    let immediate_result = backend.get(&test_key).await.expect("GET should not error");
    assert!(
        immediate_result.is_some(),
        "Value should exist immediately after SET"
    );
    println!("✓ Immediate GET successful");

    // Wait for expiration (2 seconds to be safe)
    println!("⏳ Waiting 2 seconds for TTL expiration...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify key no longer exists (returns None)
    let expired_result = backend.get(&test_key).await.expect("GET should not error");
    assert!(
        expired_result.is_none(),
        "Value should be expired after TTL"
    );
    println!("✓ Key expired correctly after TTL");
}

#[tokio::test]
async fn test_memcached_ttl_no_expiration() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 3b: No TTL (Persistent Key)");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_key = unique_test_key("no_ttl");
    let test_value = b"persistent value".to_vec();

    // Set value without TTL
    backend
        .set(&test_key, test_value.clone(), None)
        .await
        .expect("SET should succeed");

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Value should still exist
    let result = backend.get(&test_key).await.unwrap();
    assert!(result.is_some(), "Persistent key should still exist");
    println!("✓ Persistent key (no TTL) works correctly");

    // Clean up
    cleanup_keys(&backend, &[test_key]).await;
}

// =============================================================================
// Test 4: Multi-Get Operations
// =============================================================================

#[tokio::test]
async fn test_memcached_multi_get_operations() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 4: Multi-Get Operations");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    // Set 5 different keys
    let test_keys = unique_test_keys("multi", 5);

    let test_values: Vec<Vec<u8>> = vec![
        b"value1".to_vec(),
        b"value2".to_vec(),
        b"value3".to_vec(),
        b"value4".to_vec(),
        b"value5".to_vec(),
    ];

    // Set all keys
    for (key, value) in test_keys.iter().zip(test_values.iter()) {
        backend
            .set(key, value.clone(), None)
            .await
            .expect("SET should succeed");
    }
    println!("✓ Set 5 keys successfully");

    // Small delay to ensure Memcached has processed all writes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Use mget to retrieve all at once
    let test_keys_refs: Vec<&str> = test_keys.iter().map(|s| s.as_str()).collect();
    let retrieved_values = backend
        .mget(&test_keys_refs)
        .await
        .expect("MGET should not error");

    assert_eq!(retrieved_values.len(), 5, "Should retrieve 5 values");
    println!("✓ MGET retrieved 5 values");

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

    // Clean up
    cleanup_keys(&backend, &test_keys).await;
    println!("✓ Cleanup successful");
}

#[tokio::test]
async fn test_memcached_mget_with_missing_keys() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 4b: MGET with Missing Keys");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let key1 = unique_test_key("mget_exists1");
    let key2 = unique_test_key("mget_exists2");
    let key_missing = unique_test_key("mget_missing");

    // Set only some keys
    backend.set(&key1, b"value1".to_vec(), None).await.unwrap();
    backend.set(&key2, b"value2".to_vec(), None).await.unwrap();

    // MGET with mix of existing and non-existing keys
    let keys = vec![key1.as_str(), key_missing.as_str(), key2.as_str()];
    let results = backend.mget(&keys).await.unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].is_some());
    assert!(results[1].is_none()); // Missing key
    assert!(results[2].is_some());
    println!("✓ MGET handles missing keys correctly");

    // Clean up
    cleanup_keys(&backend, &[key1, key2]).await;
}

// =============================================================================
// Test 5: Flush All
// =============================================================================

#[tokio::test]
async fn test_memcached_flush_all() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test 5: Flush All (clear_all)");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    // Set several keys
    backend
        .set("test:flush:1", b"value1".to_vec(), None)
        .await
        .unwrap();
    backend
        .set("test:flush:2", b"value2".to_vec(), None)
        .await
        .unwrap();
    backend
        .set("test:flush:3", b"value3".to_vec(), None)
        .await
        .unwrap();
    println!("✓ Set 3 test keys");

    // Verify keys exist
    assert!(backend.get("test:flush:1").await.unwrap().is_some());
    assert!(backend.get("test:flush:2").await.unwrap().is_some());
    assert!(backend.get("test:flush:3").await.unwrap().is_some());
    println!("✓ Verified keys exist");

    // Call clear_all()
    backend.clear_all().await.expect("CLEAR_ALL should succeed");
    println!("✓ CLEAR_ALL executed");

    // Verify all keys are removed
    assert!(backend.get("test:flush:1").await.unwrap().is_none());
    assert!(backend.get("test:flush:2").await.unwrap().is_none());
    assert!(backend.get("test:flush:3").await.unwrap().is_none());
    println!("✓ All keys removed successfully");
}

// =============================================================================
// Additional Tests
// =============================================================================

#[tokio::test]
async fn test_memcached_delete() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test: Delete Operation");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_key = unique_test_key("delete");

    // Set key
    backend
        .set(&test_key, b"to be deleted".to_vec(), None)
        .await
        .unwrap();
    assert!(backend.exists(&test_key).await.unwrap());

    // Delete key
    backend.delete(&test_key).await.unwrap();

    // Verify deleted
    assert!(!backend.exists(&test_key).await.unwrap());
    println!("✓ DELETE operation successful");
}

#[tokio::test]
async fn test_memcached_mdelete() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test: Multi-Delete Operation");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    let test_keys = unique_test_keys("mdelete", 3);

    // Set keys with verification after each to ensure reliability
    for key in &test_keys {
        backend.set(key, b"value".to_vec(), None).await.unwrap();
        // Verify immediately after each SET
        let value = backend.get(key).await.unwrap();
        assert!(value.is_some(), "Key {} should exist after SET", key);
    }

    // Multi-delete
    let test_keys_refs: Vec<&str> = test_keys.iter().map(|s| s.as_str()).collect();
    backend
        .mdelete(&test_keys_refs)
        .await
        .expect("MDELETE should succeed");

    // Verify all deleted
    for key in &test_keys {
        let value = backend.get(key).await.unwrap();
        assert!(value.is_none(), "Key {} should be deleted", key);
    }
    println!("✓ MDELETE operation successful");
}

#[tokio::test]
async fn test_memcached_backend_clone() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test: Backend Clone");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");

    // Clone backend
    let backend1 = backend.clone();
    let backend2 = backend;

    // Both backends should work independently
    backend1
        .set("test:clone:key1", b"value1".to_vec(), None)
        .await
        .unwrap();
    backend2
        .set("test:clone:key2", b"value2".to_vec(), None)
        .await
        .unwrap();

    // Verify both keys exist
    assert!(backend1.get("test:clone:key1").await.unwrap().is_some());
    assert!(backend2.get("test:clone:key2").await.unwrap().is_some());

    println!("✓ Cloned backends work independently");

    // Clean up
    backend1.delete("test:clone:key1").await.unwrap();
    backend2.delete("test:clone:key2").await.unwrap();
}

// =============================================================================
// END-TO-END CACHE-KIT FRAMEWORK TESTS WITH MEMCACHED BACKEND
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
        "memcached_test_user"
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
        "memcached_test_product"
    }
}

// =============================================================================
// Test E2E-1: End-to-End Cache Flow with Memcached
// =============================================================================

#[tokio::test]
async fn test_e2e_cache_flow_with_memcached() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test E2E-1: End-to-End Cache Flow with Memcached Backend");

    // Setup Memcached backend
    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");
    let expander = CacheExpander::new(backend.clone());

    // Populate repository with test data
    let mut repo = InMemoryRepository::new();
    let user = User {
        id: "e2e_user_1".to_string(),
        name: "Alice Memcached".to_string(),
        email: "alice@memcached.com".to_string(),
    };
    repo.insert(user.id.clone(), user.clone());

    // First call: Cache miss → DB hit → Memcached populated
    let mut feeder = GenericFeeder::new("e2e_user_1".to_string());
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
        .await
        .expect("First cache operation should succeed");

    // Verify data was loaded from DB
    assert!(feeder.data.is_some(), "Data should be loaded from DB");
    let loaded_user = feeder.data.unwrap();
    assert_eq!(loaded_user.id, "e2e_user_1");
    assert_eq!(loaded_user.name, "Alice Memcached");
    assert_eq!(loaded_user.email, "alice@memcached.com");
    println!("✓ Cache miss → DB hit → Memcached populated");

    // Verify Memcached cache was populated
    let cache_key = "memcached_test_user:e2e_user_1";
    let cached_data = backend
        .clone()
        .get(cache_key)
        .await
        .expect("Cache get should not error");
    assert!(
        cached_data.is_some(),
        "Memcached cache should be populated after first call"
    );
    println!("✓ Memcached cache populated");

    // Second call: Memcached cache hit
    let mut feeder2 = GenericFeeder::new("e2e_user_1".to_string());
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Refresh)
        .await
        .expect("Second cache operation should succeed");

    // Verify data was loaded from Memcached cache
    assert!(
        feeder2.data.is_some(),
        "Data should be loaded from Memcached cache"
    );
    let cached_user = feeder2.data.unwrap();
    assert_eq!(cached_user, loaded_user, "Cached data should match DB data");
    println!("✓ Memcached cache hit successful");

    // Cleanup
    backend.delete(cache_key).await.ok();
}

// =============================================================================
// Test E2E-2: Multiple Entities with Memcached
// =============================================================================

#[tokio::test]
async fn test_e2e_multiple_entities_with_memcached() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test E2E-2: Multiple Entities with Memcached Backend");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");
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
        name: "Memcached Laptop".to_string(),
        price: 1499.99,
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

    // Verify both entities are cached with unique keys in Memcached
    let user_cache_key = "memcached_test_user:e2e_u1";
    let product_cache_key = "memcached_test_product:e2e_p1";

    assert!(
        backend.get(user_cache_key).await.unwrap().is_some(),
        "User should be cached in Memcached"
    );
    assert!(
        backend.get(product_cache_key).await.unwrap().is_some(),
        "Product should be cached in Memcached"
    );
    println!("✓ Multiple entity types cached in Memcached");

    // Verify cache keys are unique
    assert_ne!(user_cache_key, product_cache_key);
    println!("✓ Cache keys are unique");

    // Verify data correctness
    assert_eq!(user_feeder.data.unwrap().name, "Bob");
    assert_eq!(product_feeder.data.unwrap().name, "Memcached Laptop");
    println!("✓ No cross-contamination between entity types");

    // Cleanup
    backend.delete(user_cache_key).await.ok();
    backend.delete(product_cache_key).await.ok();
}

// =============================================================================
// Test E2E-3: Cache Strategies with Memcached
// =============================================================================

#[tokio::test]
async fn test_e2e_cache_strategies_with_memcached() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test E2E-3: Cache Strategies with Memcached Backend");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");
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
    backend
        .clone()
        .delete("memcached_test_user:e2e_strategy")
        .await
        .ok();
}

// =============================================================================
// Test E2E-4: TTL with Memcached Backend
// =============================================================================

#[tokio::test]
async fn test_e2e_ttl_with_memcached() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test E2E-4: TTL with Memcached Backend");

    use cache_kit::observability::TtlPolicy;

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");
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
    backend
        .clone()
        .delete("memcached_test_user:e2e_ttl")
        .await
        .ok();
}

// =============================================================================
// Test E2E-5: Concurrent Operations with Memcached
// =============================================================================

#[tokio::test]
async fn test_e2e_concurrent_operations_with_memcached() {
    if !is_memcached_available().await {
        println!("⚠️  Memcached not available, skipping test");
        return;
    }

    println!("Test E2E-5: Concurrent Operations with Memcached Backend");

    let backend = create_test_backend()
        .await
        .expect("Failed to create Memcached backend");
    let expander = CacheExpander::new(backend.clone());

    // Shared repository with 10 users
    let repo = {
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
    };

    // Perform cache operations sequentially but test concurrent-like scenarios
    // (Memcached backend is thread-safe internally)
    for i in 0..10 {
        for j in 0..3 {
            let user_id = format!("e2e_concurrent_{}", (i + j) % 10);
            let mut feeder = GenericFeeder::new(user_id);

            let result = expander
                .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
                .await;

            assert!(result.is_ok(), "Cache operation should succeed");

            // Verify data is correct
            if let Some(user) = feeder.data {
                assert!(user.name.starts_with("Concurrent User "));
                assert!(user.email.contains("@concurrent.com"));
            }
        }
    }

    println!("✓ 10 iterations completed 30 total operations");

    // Verify all users are cached in Memcached
    let mut cached_count = 0;
    for i in 0..10 {
        let cache_key = format!("memcached_test_user:e2e_concurrent_{}", i);
        if backend.exists(&cache_key).await.unwrap() {
            cached_count += 1;
            // Cleanup
            backend.delete(&cache_key).await.ok();
        }
    }

    assert!(
        cached_count > 0,
        "At least some users should be cached after cache operations"
    );
    println!("✓ {} users cached successfully", cached_count);
    println!("✓ Cache operations completed successfully");
}
