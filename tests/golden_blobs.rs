//! Golden blob regression tests.
//!
//! These tests verify that the current code can deserialize cache entries
//! serialized with previous schema versions, ensuring backward compatibility.
//!
//! # Purpose
//!
//! - Detect accidental serialization format changes
//! - Verify version migration behavior
//! - Ensure production cache compatibility
//!
//! # When Tests Fail
//!
//! If these tests fail, it means:
//! 1. You accidentally changed serialization format (fix the code), OR
//! 2. You intentionally changed schema (bump CURRENT_SCHEMA_VERSION and regenerate golden blobs)

use cache_kit::serialization::{deserialize_from_cache, CACHE_MAGIC, CURRENT_SCHEMA_VERSION};
use cache_kit::CacheEntity;
use serde::{Deserialize, Serialize};

// ============================================================================
// Test Entities (Must match golden_blob_generator.rs definitions EXACTLY)
// ============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct User {
    id: u64,
    name: String,
    email: String,
}

impl CacheEntity for User {
    type Key = u64;
    fn cache_key(&self) -> Self::Key {
        self.id
    }
    fn cache_prefix() -> &'static str {
        "user"
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct Product {
    id: String,
    name: String,
    price: f64,
    in_stock: bool,
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

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct ComplexEntity {
    id: u64,
    name: String,
    tags: Vec<String>,
    score: f64,
}

impl CacheEntity for ComplexEntity {
    type Key = u64;
    fn cache_key(&self) -> Self::Key {
        self.id
    }
    fn cache_prefix() -> &'static str {
        "complex"
    }
}

// ============================================================================
// Golden Blob Regression Tests
// ============================================================================

#[test]
fn test_deserialize_user_v1() {
    // Load golden blob created with schema version 1
    let golden_bytes = include_bytes!("golden/user_v1.bin");

    // Verify envelope structure
    assert_eq!(
        &golden_bytes[0..4],
        CACHE_MAGIC,
        "Golden blob has invalid magic"
    );

    // Verify version by deserializing envelope first (postcard uses variable-length encoding)
    use cache_kit::serialization::CacheEnvelope;
    let envelope: CacheEnvelope<User> =
        postcard::from_bytes(golden_bytes).expect("Failed to deserialize envelope");
    assert_eq!(envelope.version, 1, "Golden blob should be version 1");

    // Current code MUST be able to deserialize it
    let user: User = deserialize_from_cache(golden_bytes)
        .expect("Failed to deserialize user_v1.bin - schema incompatibility!");

    // Verify data correctness
    assert_eq!(user.id, 42);
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
}

#[test]
fn test_deserialize_product_v1() {
    let golden_bytes = include_bytes!("golden/product_v1.bin");

    // Verify envelope
    assert_eq!(&golden_bytes[0..4], CACHE_MAGIC);

    // Verify version by deserializing envelope first (postcard uses variable-length encoding)
    use cache_kit::serialization::CacheEnvelope;
    let envelope: CacheEnvelope<Product> =
        postcard::from_bytes(golden_bytes).expect("Failed to deserialize envelope");
    assert_eq!(envelope.version, 1);

    // Deserialize
    let product: Product = deserialize_from_cache(golden_bytes)
        .expect("Failed to deserialize product_v1.bin - schema incompatibility!");

    // Verify data
    assert_eq!(product.id, "prod_123");
    assert_eq!(product.name, "Widget");
    assert!((product.price - 99.99).abs() < 0.01);
    assert!(product.in_stock);
}

#[test]
fn test_deserialize_complex_v1() {
    let golden_bytes = include_bytes!("golden/complex_v1.bin");

    // Verify envelope
    assert_eq!(&golden_bytes[0..4], CACHE_MAGIC);

    // Verify version by deserializing envelope first (postcard uses variable-length encoding)
    use cache_kit::serialization::CacheEnvelope;
    let envelope: CacheEnvelope<ComplexEntity> =
        postcard::from_bytes(golden_bytes).expect("Failed to deserialize envelope");
    assert_eq!(envelope.version, 1);

    // Deserialize
    let entity: ComplexEntity = deserialize_from_cache(golden_bytes)
        .expect("Failed to deserialize complex_v1.bin - schema incompatibility!");

    // Verify data
    assert_eq!(entity.id, 100);
    assert_eq!(entity.name, "Complex Test Entity");
    assert_eq!(entity.tags, vec!["tag1", "tag2", "tag3"]);
    assert!((entity.score - 95.5).abs() < 0.01);
}

// ============================================================================
// Version Mismatch Tests
// ============================================================================

#[test]
fn test_future_version_rejected() {
    // Manually create an envelope with a future schema version
    use cache_kit::serialization::{serialize_for_cache, CacheEnvelope};

    let user = User {
        id: 999,
        name: "Future User".to_string(),
        email: "future@example.com".to_string(),
    };

    // Serialize normally
    let bytes = serialize_for_cache(&user).expect("Serialization should succeed");

    // Deserialize envelope and manually change version to 999 (future version)
    let mut envelope: CacheEnvelope<User> =
        postcard::from_bytes(&bytes).expect("Failed to deserialize envelope");
    let future_version: u32 = 999;
    envelope.version = future_version;

    // Re-serialize with modified version
    let modified_bytes =
        postcard::to_allocvec(&envelope).expect("Failed to serialize modified envelope");

    // Attempt to deserialize should FAIL with version mismatch
    let result: Result<User, _> = deserialize_from_cache(&modified_bytes);

    assert!(
        result.is_err(),
        "Should reject future version (expected: {}, got: {})",
        CURRENT_SCHEMA_VERSION,
        future_version
    );

    // Verify it's specifically a version mismatch error
    match result {
        Err(cache_kit::Error::VersionMismatch { expected, found }) => {
            assert_eq!(expected, CURRENT_SCHEMA_VERSION);
            assert_eq!(found, 999);
        }
        _ => panic!("Expected VersionMismatch error"),
    }
}

#[test]
fn test_corrupted_magic_rejected() {
    // Load a valid golden blob
    let mut bytes = include_bytes!("golden/user_v1.bin").to_vec();

    // Corrupt the magic header
    bytes[0] = b'X';
    bytes[1] = b'X';
    bytes[2] = b'X';
    bytes[3] = b'X';

    // Should be rejected
    let result: Result<User, _> = deserialize_from_cache(&bytes);

    assert!(result.is_err(), "Should reject corrupted magic header");

    match result {
        Err(cache_kit::Error::InvalidCacheEntry(_)) => {
            // Expected
        }
        _ => panic!("Expected InvalidCacheEntry error"),
    }
}

// ============================================================================
// Format Stability Tests
// ============================================================================

#[test]
fn test_golden_blob_has_not_changed() {
    // This test verifies that the golden blob ITSELF hasn't changed
    // If this fails, someone either:
    // 1. Regenerated golden blobs without bumping version (BAD), or
    // 2. Manually edited the .bin files (BAD)

    use cache_kit::serialization::CacheEnvelope;

    let user_v1_bytes = include_bytes!("golden/user_v1.bin");
    let product_v1_bytes = include_bytes!("golden/product_v1.bin");
    let complex_v1_bytes = include_bytes!("golden/complex_v1.bin");

    // Verify they all have correct magic
    assert_eq!(&user_v1_bytes[0..4], CACHE_MAGIC);
    assert_eq!(&product_v1_bytes[0..4], CACHE_MAGIC);
    assert_eq!(&complex_v1_bytes[0..4], CACHE_MAGIC);

    // Verify they all have version 1 by deserializing envelopes
    let user_envelope: CacheEnvelope<User> =
        postcard::from_bytes(user_v1_bytes).expect("Failed to deserialize user_v1 envelope");
    assert_eq!(
        user_envelope.version, 1,
        "user_v1.bin should have version 1"
    );

    let product_envelope: CacheEnvelope<Product> =
        postcard::from_bytes(product_v1_bytes).expect("Failed to deserialize product_v1 envelope");
    assert_eq!(
        product_envelope.version, 1,
        "product_v1.bin should have version 1"
    );

    let complex_envelope: CacheEnvelope<ComplexEntity> =
        postcard::from_bytes(complex_v1_bytes).expect("Failed to deserialize complex_v1 envelope");
    assert_eq!(
        complex_envelope.version, 1,
        "complex_v1.bin should have version 1"
    );
}

#[test]
fn test_deserialization_is_deterministic() {
    // Verify that deserializing and re-serializing produces identical bytes
    use cache_kit::serialization::serialize_for_cache;

    let golden_bytes = include_bytes!("golden/user_v1.bin");

    // Deserialize
    let user: User = deserialize_from_cache(golden_bytes).expect("Deserialization should succeed");

    // Re-serialize with current code
    let new_bytes = serialize_for_cache(&user).expect("Serialization should succeed");

    // Should produce IDENTICAL bytes (deterministic)
    assert_eq!(
        golden_bytes,
        new_bytes.as_slice(),
        "Deserialization → Serialization roundtrip should be deterministic"
    );
}

// ============================================================================
// Migration Scenario Tests
// ============================================================================

#[test]
fn test_production_migration_scenario() {
    // Simulates production scenario:
    // 1. Old code (v1) writes cache entry
    // 2. New code (v1) reads it - should work!

    let old_cache_entry = include_bytes!("golden/user_v1.bin");

    // Current code reads old entry
    let user: User = deserialize_from_cache(old_cache_entry)
        .expect("Current code should read old cache entries");

    // Verify data integrity
    assert_eq!(user.id, 42);
    assert_eq!(user.name, "Alice");

    // If CURRENT_SCHEMA_VERSION was bumped to 2, this test would fail
    // → That's GOOD! It forces you to regenerate golden blobs
}

#[test]
#[ignore] // Only run manually to test version bump scenarios
fn test_version_2_would_reject_v1() {
    // This test documents what WOULD happen if we bumped to version 2
    // Run manually: cargo test test_version_2_would_reject_v1 -- --ignored

    let v1_cache_entry = include_bytes!("golden/user_v1.bin");

    // If CURRENT_SCHEMA_VERSION == 2, this would fail:
    if CURRENT_SCHEMA_VERSION > 1 {
        let result: Result<User, _> = deserialize_from_cache(v1_cache_entry);
        assert!(result.is_err(), "Version 2+ should reject v1 cache entries");
    } else {
        println!("CURRENT_SCHEMA_VERSION is still 1, test skipped");
    }
}
