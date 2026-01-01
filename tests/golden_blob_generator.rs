//! Golden blob generator - creates reference serialized files.
//!
//! Run this to regenerate golden blobs after intentional schema changes:
//! ```bash
//! cargo test --test golden_blob_generator -- --nocapture
//! ```

use cache_kit::serialization::serialize_for_cache;
use cache_kit::CacheEntity;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ============================================================================
// Test Entities (Must match golden_blobs.rs definitions)
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
// Generator Functions
// ============================================================================

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn generate_user_v1() -> std::io::Result<()> {
    let user = User {
        id: 42,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let bytes = serialize_for_cache(&user).expect("Serialization should succeed");

    let path = golden_dir().join("user_v1.bin");
    fs::write(&path, &bytes)?;

    println!("✅ Generated: {}", path.display());
    println!("   Size: {} bytes", bytes.len());
    println!("   Checksum: {}", md5_hash(&bytes));
    println!();

    Ok(())
}

fn generate_product_v1() -> std::io::Result<()> {
    let product = Product {
        id: "prod_123".to_string(),
        name: "Widget".to_string(),
        price: 99.99,
        in_stock: true,
    };

    let bytes = serialize_for_cache(&product).expect("Serialization should succeed");

    let path = golden_dir().join("product_v1.bin");
    fs::write(&path, &bytes)?;

    println!("✅ Generated: {}", path.display());
    println!("   Size: {} bytes", bytes.len());
    println!("   Checksum: {}", md5_hash(&bytes));
    println!();

    Ok(())
}

fn generate_complex_v1() -> std::io::Result<()> {
    let entity = ComplexEntity {
        id: 100,
        name: "Complex Test Entity".to_string(),
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        score: 95.5,
    };

    let bytes = serialize_for_cache(&entity).expect("Serialization should succeed");

    let path = golden_dir().join("complex_v1.bin");
    fs::write(&path, &bytes)?;

    println!("✅ Generated: {}", path.display());
    println!("   Size: {} bytes", bytes.len());
    println!("   Checksum: {}", md5_hash(&bytes));
    println!();

    Ok(())
}

// Simple MD5 hash for verification
fn md5_hash(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ============================================================================
// Test Runner
// ============================================================================

#[test]
fn generate_all_golden_blobs() {
    println!("\n🔧 Generating golden blob files...\n");

    // Ensure directory exists
    fs::create_dir_all(golden_dir()).expect("Failed to create golden directory");

    // Generate all blobs
    generate_user_v1().expect("Failed to generate user_v1.bin");
    generate_product_v1().expect("Failed to generate product_v1.bin");
    generate_complex_v1().expect("Failed to generate complex_v1.bin");

    println!("✅ All golden blobs generated successfully!");
    println!("\nNext steps:");
    println!("1. Verify tests pass: cargo test --test golden_blobs");
    println!("2. Commit the .bin files to version control");
    println!("3. Update CHANGELOG with schema version bump");
}

#[test]
fn verify_golden_blobs_exist() {
    let golden_files = vec!["user_v1.bin", "product_v1.bin", "complex_v1.bin"];

    for file in golden_files {
        let path = golden_dir().join(file);
        if !path.exists() {
            panic!(
                "Golden blob missing: {}\nRun: cargo test --test golden_blob_generator -- --nocapture",
                path.display()
            );
        }
    }
}
