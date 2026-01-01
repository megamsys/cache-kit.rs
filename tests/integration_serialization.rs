//! Integration tests for cache serialization with real backends.
//!
//! These tests verify that the Postcard serialization with versioned envelopes
//! works correctly across different cache backends (InMemory, Redis, Memcached).

use cache_kit::backend::{CacheBackend, InMemoryBackend};
use cache_kit::feed::GenericFeeder;
use cache_kit::repository::InMemoryRepository;
use cache_kit::serialization::{
    deserialize_from_cache, serialize_for_cache, CACHE_MAGIC, CURRENT_SCHEMA_VERSION,
};
use cache_kit::{CacheEntity, CacheExpander, CacheStrategy};
use serde::{Deserialize, Serialize};

// ============================================================================
// Test Entities
// ============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
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

// ============================================================================
// InMemory Backend Tests
// ============================================================================

#[tokio::test]
async fn test_inmemory_backend_postcard_roundtrip() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    // Setup repository with test data
    let mut repo = InMemoryRepository::new();
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        active: true,
    };
    repo.insert(user.id, user.clone());

    // First call: cache miss -> DB hit -> cache populated
    let mut feeder = GenericFeeder::new(1u64);
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
        .await
        .unwrap();

    assert_eq!(feeder.data, Some(user.clone()));

    // Second call: cache hit
    let mut feeder2 = GenericFeeder::new(1u64);
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Fresh)
        .await
        .unwrap();

    assert_eq!(feeder2.data, Some(user));
}

#[tokio::test]
async fn test_inmemory_backend_multiple_entities() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    let mut repo = InMemoryRepository::new();

    let user1 = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        active: true,
    };

    let user2 = User {
        id: 2,
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
        active: false,
    };

    repo.insert(user1.id, user1.clone());
    repo.insert(user2.id, user2.clone());

    // Cache both users
    let mut feeder1 = GenericFeeder::new(1u64);
    expander
        .with::<User, _, _>(&mut feeder1, &repo, CacheStrategy::Refresh)
        .await
        .unwrap();

    let mut feeder2 = GenericFeeder::new(2u64);
    expander
        .with::<User, _, _>(&mut feeder2, &repo, CacheStrategy::Refresh)
        .await
        .unwrap();

    assert_eq!(feeder1.data, Some(user1));
    assert_eq!(feeder2.data, Some(user2));
}

#[tokio::test]
async fn test_inmemory_backend_different_entity_types() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    let mut user_repo = InMemoryRepository::new();
    let mut product_repo = InMemoryRepository::new();

    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        active: true,
    };

    let product = Product {
        id: "prod_123".to_string(),
        name: "Widget".to_string(),
        price: 99.99,
        in_stock: true,
    };

    user_repo.insert(user.id, user.clone());
    product_repo.insert(product.id.clone(), product.clone());

    // Cache different entity types
    let mut user_feeder = GenericFeeder::new(1u64);
    expander
        .with::<User, _, _>(&mut user_feeder, &user_repo, CacheStrategy::Refresh)
        .await
        .unwrap();

    let mut product_feeder = GenericFeeder::new("prod_123".to_string());
    expander
        .with::<Product, _, _>(&mut product_feeder, &product_repo, CacheStrategy::Refresh)
        .await
        .unwrap();

    assert_eq!(user_feeder.data, Some(user));
    assert_eq!(product_feeder.data, Some(product));
}

#[tokio::test]
async fn test_inmemory_backend_cache_miss() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend);

    let repo: InMemoryRepository<User> = InMemoryRepository::new();

    // Try to get non-existent user
    let mut feeder = GenericFeeder::new(999u64);
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Fresh)
        .await
        .unwrap();

    assert_eq!(feeder.data, None);
}

// ============================================================================
// Direct Serialization Tests (Backend-agnostic)
// ============================================================================

#[tokio::test]
async fn test_direct_serialization_envelope_format() {
    let user = User {
        id: 42,
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        active: true,
    };

    // Serialize
    let bytes = serialize_for_cache(&user).unwrap();

    // Verify envelope structure
    assert!(
        bytes.len() > 8,
        "Envelope should be at least 8 bytes (magic + version)"
    );

    // Verify magic
    let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
    assert_eq!(magic, CACHE_MAGIC);

    // Verify version by deserializing envelope (postcard uses variable-length encoding)
    use cache_kit::serialization::CacheEnvelope;
    let envelope: CacheEnvelope<User> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(envelope.version, CURRENT_SCHEMA_VERSION);

    // Verify roundtrip
    let deserialized: User = deserialize_from_cache(&bytes).unwrap();
    assert_eq!(deserialized, user);
}

#[tokio::test]
async fn test_serialization_consistency_across_calls() {
    let user = User {
        id: 100,
        name: "Consistent User".to_string(),
        email: "consistent@example.com".to_string(),
        active: true,
    };

    // Serialize multiple times
    let bytes1 = serialize_for_cache(&user).unwrap();
    let bytes2 = serialize_for_cache(&user).unwrap();
    let bytes3 = serialize_for_cache(&user).unwrap();

    // All should be identical (deterministic)
    assert_eq!(bytes1, bytes2);
    assert_eq!(bytes2, bytes3);
}

#[tokio::test]
async fn test_serialization_size_comparison_with_json() {
    let user = User {
        id: 1,
        name: "Size Test User".to_string(),
        email: "size@example.com".to_string(),
        active: true,
    };

    // Postcard with envelope
    let postcard_bytes = serialize_for_cache(&user).unwrap();

    // JSON (for comparison)
    let json_bytes = serde_json::to_vec(&user).unwrap();

    // Postcard should be smaller or similar size
    // (With envelope overhead, might be close, but typically still smaller)
    println!("Postcard size: {} bytes", postcard_bytes.len());
    println!("JSON size: {} bytes", json_bytes.len());

    // For this small struct, Postcard should be competitive
    assert!(
        postcard_bytes.len() < json_bytes.len() * 2,
        "Postcard should not be more than 2x larger than JSON"
    );
}

#[tokio::test]
async fn test_serialization_complex_data() {
    #[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
    struct ComplexEntity {
        id: u64,
        name: String,
        tags: Vec<String>,
        metadata: std::collections::HashMap<String, String>,
        score: f64,
        active: bool,
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

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());
    metadata.insert("key2".to_string(), "value2".to_string());

    let entity = ComplexEntity {
        id: 1,
        name: "Complex Entity".to_string(),
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        metadata,
        score: 95.5,
        active: true,
    };

    // Roundtrip through serialization
    let bytes = serialize_for_cache(&entity).unwrap();
    let deserialized: ComplexEntity = deserialize_from_cache(&bytes).unwrap();

    assert_eq!(deserialized, entity);
}

// ============================================================================
// Cache Backend Integration Tests
// ============================================================================

#[tokio::test]
async fn test_backend_raw_bytes_validation() {
    let backend = InMemoryBackend::new();

    let user = User {
        id: 1,
        name: "Raw Test".to_string(),
        email: "raw@example.com".to_string(),
        active: true,
    };

    // Serialize user
    let bytes = serialize_for_cache(&user).unwrap();

    // Store raw bytes in backend
    let key = format!("{}:{}", User::cache_prefix(), user.cache_key());
    backend.set(&key, bytes.clone(), None).await.unwrap();

    // Retrieve raw bytes
    let retrieved_bytes = backend.get(&key).await.unwrap().expect("Should find entry");

    // Verify envelope in raw bytes
    assert_eq!(&retrieved_bytes[0..4], b"CKIT");

    // Deserialize
    let deserialized: User = deserialize_from_cache(&retrieved_bytes).unwrap();
    assert_eq!(deserialized, user);
}

#[tokio::test]
async fn test_backend_stores_postcard_not_json() {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend.clone());

    let user = User {
        id: 1,
        name: "Format Test".to_string(),
        email: "format@example.com".to_string(),
        active: true,
    };

    // Use expander to cache user
    let mut repo = InMemoryRepository::new();
    repo.insert(user.id, user.clone());

    let mut feeder = GenericFeeder::new(1u64);
    expander
        .with::<User, _, _>(&mut feeder, &repo, CacheStrategy::Refresh)
        .await
        .unwrap();

    // Get raw bytes from backend
    let key = format!("{}:{}", User::cache_prefix(), user.cache_key());
    let raw_bytes = backend.get(&key).await.unwrap().expect("Should find entry");

    // Verify it's NOT JSON (should start with CKIT magic, not '{')
    assert_eq!(&raw_bytes[0..4], b"CKIT");
    assert_ne!(raw_bytes[0], b'{'); // NOT JSON

    // Verify it IS valid Postcard with envelope
    let deserialized: User = deserialize_from_cache(&raw_bytes).unwrap();
    assert_eq!(deserialized, user);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[tokio::test]
async fn test_empty_string_fields() {
    let user = User {
        id: 1,
        name: String::new(),
        email: String::new(),
        active: false,
    };

    let bytes = serialize_for_cache(&user).unwrap();
    let deserialized: User = deserialize_from_cache(&bytes).unwrap();

    assert_eq!(deserialized, user);
}

#[tokio::test]
async fn test_large_string_fields() {
    let user = User {
        id: 1,
        name: "x".repeat(10000),
        email: "y".repeat(5000),
        active: true,
    };

    let bytes = serialize_for_cache(&user).unwrap();
    let deserialized: User = deserialize_from_cache(&bytes).unwrap();

    assert_eq!(deserialized, user);
}

#[tokio::test]
async fn test_special_characters_in_strings() {
    let user = User {
        id: 1,
        name: "User with émojis 🎉 and spëcial çhars".to_string(),
        email: "test+tag@example.com".to_string(),
        active: true,
    };

    let bytes = serialize_for_cache(&user).unwrap();
    let deserialized: User = deserialize_from_cache(&bytes).unwrap();

    assert_eq!(deserialized, user);
}

#[tokio::test]
async fn test_max_values() {
    let user = User {
        id: u64::MAX,
        name: "Max User".to_string(),
        email: "max@example.com".to_string(),
        active: true,
    };

    let bytes = serialize_for_cache(&user).unwrap();
    let deserialized: User = deserialize_from_cache(&bytes).unwrap();

    assert_eq!(deserialized, user);
}
