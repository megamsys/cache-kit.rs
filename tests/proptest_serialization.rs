//! Property-based tests for cache serialization.
//!
//! These tests use proptest to verify that serialization properties hold
//! for randomly generated inputs, catching edge cases that example-based
//! tests might miss.
//!
//! # Properties Tested
//!
//! 1. **Roundtrip Property**: deserialize(serialize(x)) == x for ANY x
//! 2. **Determinism Property**: serialize(x) == serialize(x) always
//! 3. **Envelope Property**: All serialized data has correct magic + version
//! 4. **Size Property**: Postcard is competitive with JSON

use cache_kit::serialization::{
    deserialize_from_cache, serialize_for_cache, CACHE_MAGIC, CURRENT_SCHEMA_VERSION,
};
use cache_kit::CacheEntity;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Test Entities with Arbitrary Implementations
// ============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
    active: bool,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Product {
    id: String,
    name: String,
    price: f64,
    in_stock: bool,
    quantity: i32,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ComplexEntity {
    id: u64,
    name: String,
    tags: Vec<String>,
    score: f64,
    active: bool,
    count: i64,
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
// Arbitrary Implementations (for property-based testing)
// ============================================================================

/// Generate arbitrary User with any valid values
fn arb_user() -> impl Strategy<Value = User> {
    (
        any::<u64>(),
        any::<String>(),
        any::<String>(),
        any::<bool>(),
    )
        .prop_map(|(id, name, email, active)| User {
            id,
            name,
            email,
            active,
        })
}

/// Generate arbitrary Product with any valid values
fn arb_product() -> impl Strategy<Value = Product> {
    (
        any::<String>(),
        any::<String>(),
        any::<f64>(),
        any::<bool>(),
        any::<i32>(),
    )
        .prop_map(|(id, name, price, in_stock, quantity)| Product {
            id,
            name,
            price,
            in_stock,
            quantity,
        })
}

/// Generate arbitrary ComplexEntity with collections
fn arb_complex_entity() -> impl Strategy<Value = ComplexEntity> {
    (
        any::<u64>(),
        any::<String>(),
        prop::collection::vec(any::<String>(), 0..10),
        any::<f64>(),
        any::<bool>(),
        any::<i64>(),
    )
        .prop_map(|(id, name, tags, score, active, count)| ComplexEntity {
            id,
            name,
            tags,
            score,
            active,
            count,
        })
}

// ============================================================================
// Property 1: Roundtrip Property
// ============================================================================

proptest! {
    /// Property: For any User, deserialize(serialize(user)) == user
    #[test]
    fn prop_user_roundtrip(user in arb_user()) {
        let bytes = serialize_for_cache(&user)
            .expect("Serialization should never fail for valid User");

        let deserialized: User = deserialize_from_cache(&bytes)
            .expect("Deserialization should never fail for valid bytes");

        prop_assert_eq!(user, deserialized);
    }

    /// Property: For any Product, roundtrip preserves data
    #[test]
    fn prop_product_roundtrip(product in arb_product()) {
        let bytes = serialize_for_cache(&product)
            .expect("Serialization should never fail for valid Product");

        let deserialized: Product = deserialize_from_cache(&bytes)
            .expect("Deserialization should never fail for valid bytes");

        prop_assert_eq!(product, deserialized);
    }

    /// Property: For any ComplexEntity with collections, roundtrip works
    #[test]
    fn prop_complex_entity_roundtrip(entity in arb_complex_entity()) {
        let bytes = serialize_for_cache(&entity)
            .expect("Serialization should never fail for valid ComplexEntity");

        let deserialized: ComplexEntity = deserialize_from_cache(&bytes)
            .expect("Deserialization should never fail for valid bytes");

        prop_assert_eq!(entity, deserialized);
    }
}

// ============================================================================
// Property 2: Determinism Property
// ============================================================================

proptest! {
    /// Property: Serializing the same User twice produces identical bytes
    #[test]
    fn prop_user_determinism(user in arb_user()) {
        let bytes1 = serialize_for_cache(&user)
            .expect("Serialization should succeed");
        let bytes2 = serialize_for_cache(&user)
            .expect("Serialization should succeed");

        prop_assert_eq!(bytes1, bytes2, "Serialization must be deterministic");
    }

    /// Property: Determinism holds for Products too
    #[test]
    fn prop_product_determinism(product in arb_product()) {
        let bytes1 = serialize_for_cache(&product)
            .expect("Serialization should succeed");
        let bytes2 = serialize_for_cache(&product)
            .expect("Serialization should succeed");

        prop_assert_eq!(bytes1, bytes2, "Serialization must be deterministic");
    }

    /// Property: Determinism for complex entities with collections
    #[test]
    fn prop_complex_determinism(entity in arb_complex_entity()) {
        let bytes1 = serialize_for_cache(&entity)
            .expect("Serialization should succeed");
        let bytes2 = serialize_for_cache(&entity)
            .expect("Serialization should succeed");

        prop_assert_eq!(bytes1, bytes2, "Serialization must be deterministic");
    }
}

// ============================================================================
// Property 3: Envelope Format Property
// ============================================================================

proptest! {
    /// Property: All serialized Users have correct envelope
    #[test]
    fn prop_user_envelope_format(user in arb_user()) {
        use cache_kit::serialization::CacheEnvelope;

        let bytes = serialize_for_cache(&user)
            .expect("Serialization should succeed");

        // Must have at least magic (4) bytes
        prop_assert!(bytes.len() >= 4, "Envelope too small: {} bytes", bytes.len());

        // Check magic header
        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        prop_assert_eq!(magic, CACHE_MAGIC, "Invalid magic header");

        // Check version by deserializing envelope (postcard uses variable-length encoding)
        let envelope: CacheEnvelope<User> = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize envelope");
        prop_assert_eq!(envelope.version, CURRENT_SCHEMA_VERSION, "Invalid schema version");
    }

    /// Property: All serialized Products have correct envelope
    #[test]
    fn prop_product_envelope_format(product in arb_product()) {
        use cache_kit::serialization::CacheEnvelope;

        let bytes = serialize_for_cache(&product)
            .expect("Serialization should succeed");

        prop_assert!(bytes.len() >= 4);

        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        prop_assert_eq!(magic, CACHE_MAGIC);

        // Check version by deserializing envelope (postcard uses variable-length encoding)
        let envelope: CacheEnvelope<Product> = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize envelope");
        prop_assert_eq!(envelope.version, CURRENT_SCHEMA_VERSION);
    }

    /// Property: Envelope format holds for any entity
    #[test]
    fn prop_complex_envelope_format(entity in arb_complex_entity()) {
        use cache_kit::serialization::CacheEnvelope;

        let bytes = serialize_for_cache(&entity)
            .expect("Serialization should succeed");

        prop_assert!(bytes.len() >= 4);

        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        prop_assert_eq!(magic, CACHE_MAGIC);

        // Check version by deserializing envelope (postcard uses variable-length encoding)
        let envelope: CacheEnvelope<ComplexEntity> = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize envelope");
        prop_assert_eq!(envelope.version, CURRENT_SCHEMA_VERSION);
    }
}

// ============================================================================
// Property 4: Size Efficiency Property
// ============================================================================

proptest! {
    /// Property: Postcard is competitive with JSON size
    ///
    /// Note: Postcard might be slightly larger for very small structs due to
    /// envelope overhead (8 bytes), but should be smaller for larger structs
    #[test]
    fn prop_user_size_efficiency(user in arb_user()) {
        let postcard_bytes = serialize_for_cache(&user)
            .expect("Postcard serialization should succeed");

        let json_bytes = serde_json::to_vec(&user)
            .expect("JSON serialization should succeed");

        // For larger data, Postcard should be smaller
        // For tiny data, envelope overhead might make it larger
        // Property: Postcard should never be MORE than 2x JSON size
        prop_assert!(
            postcard_bytes.len() < json_bytes.len() * 2,
            "Postcard too large: {} bytes vs JSON {} bytes (ratio: {:.2}x)",
            postcard_bytes.len(),
            json_bytes.len(),
            postcard_bytes.len() as f64 / json_bytes.len() as f64
        );
    }
}

// ============================================================================
// Property 5: Edge Cases Property
// ============================================================================

proptest! {
    /// Property: Empty strings are handled correctly
    #[test]
    fn prop_empty_strings_work(id in any::<u64>(), active in any::<bool>()) {
        let user = User {
            id,
            name: String::new(),
            email: String::new(),
            active,
        };

        let bytes = serialize_for_cache(&user)?;
        let deserialized: User = deserialize_from_cache(&bytes)?;

        prop_assert_eq!(user, deserialized);
    }

    /// Property: Maximum u64 values work
    #[test]
    fn prop_max_u64_works(name in any::<String>(), email in any::<String>()) {
        let user = User {
            id: u64::MAX,
            name,
            email,
            active: true,
        };

        let bytes = serialize_for_cache(&user)?;
        let deserialized: User = deserialize_from_cache(&bytes)?;

        prop_assert_eq!(user, deserialized);
    }

    /// Property: Minimum i64 values work
    #[test]
    fn prop_min_i64_works(
        id in any::<u64>(),
        name in any::<String>(),
        tags in prop::collection::vec(any::<String>(), 0..5)
    ) {
        let entity = ComplexEntity {
            id,
            name,
            tags,
            score: 0.0,
            active: false,
            count: i64::MIN,
        };

        let bytes = serialize_for_cache(&entity)?;
        let deserialized: ComplexEntity = deserialize_from_cache(&bytes)?;

        prop_assert_eq!(entity, deserialized);
    }

    /// Property: Large collections work
    #[test]
    fn prop_large_collections_work(
        id in any::<u64>(),
        name in any::<String>(),
        tags in prop::collection::vec(any::<String>(), 0..1000)  // Up to 1000 items
    ) {
        let entity = ComplexEntity {
            id,
            name,
            tags,
            score: 0.0,
            active: true,
            count: 0,
        };

        let bytes = serialize_for_cache(&entity)?;
        let deserialized: ComplexEntity = deserialize_from_cache(&bytes)?;

        prop_assert_eq!(entity, deserialized);
    }

    /// Property: Special float values (NaN, Infinity) work
    #[test]
    fn prop_special_floats_work(
        id in prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap(),
        name in any::<String>()
    ) {
        // Test NaN
        let product_nan = Product {
            id: id.clone(),
            name: name.clone(),
            price: f64::NAN,
            in_stock: true,
            quantity: 10,
        };

        let bytes = serialize_for_cache(&product_nan)?;
        let deserialized: Product = deserialize_from_cache(&bytes)?;
        prop_assert!(deserialized.price.is_nan());

        // Test Infinity
        let product_inf = Product {
            id,
            name,
            price: f64::INFINITY,
            in_stock: false,
            quantity: 0,
        };

        let bytes = serialize_for_cache(&product_inf)?;
        let deserialized: Product = deserialize_from_cache(&bytes)?;
        prop_assert_eq!(deserialized.price, f64::INFINITY);
    }
}

// ============================================================================
// Property 6: Corruption Detection Property
// ============================================================================

proptest! {
    /// Property: Corrupted magic is always detected
    #[test]
    fn prop_corrupted_magic_detected(user in arb_user()) {
        let mut bytes = serialize_for_cache(&user)
            .expect("Serialization should succeed");

        // Corrupt the magic header
        bytes[0] = b'X';
        bytes[1] = b'X';
        bytes[2] = b'X';
        bytes[3] = b'X';

        let result: Result<User, _> = deserialize_from_cache(&bytes);
        prop_assert!(result.is_err(), "Should reject corrupted magic");
    }

    /// Property: Wrong version is always detected
    #[test]
    fn prop_wrong_version_detected(product in arb_product()) {
        let bytes = serialize_for_cache(&product)
            .expect("Serialization should succeed");

        // Manually create envelope with wrong version
        let mut corrupted = bytes.clone();
        // Overwrite version (bytes 4-7) with version 999
        corrupted[4..8].copy_from_slice(&999u32.to_le_bytes());

        let result: Result<Product, _> = deserialize_from_cache(&corrupted);
        prop_assert!(result.is_err(), "Should reject wrong version");
    }

    /// Property: Truncated data is always detected
    #[test]
    fn prop_truncated_data_detected(entity in arb_complex_entity()) {
        let bytes = serialize_for_cache(&entity)
            .expect("Serialization should succeed");

        if bytes.len() > 20 {
            // Truncate the payload
            let truncated = &bytes[..bytes.len() / 2];

            let result: Result<ComplexEntity, _> = deserialize_from_cache(truncated);
            prop_assert!(result.is_err(), "Should reject truncated data");
        }
    }
}
