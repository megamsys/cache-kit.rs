//! Example demonstrating advanced configuration with builder pattern.

use cache_kit::{
    backend::InMemoryBackend,
    error::Result,
    observability::{CacheMetrics, TtlPolicy},
    strategy::CacheStrategy,
    CacheEntity, CacheExpander, CacheFeed, DataRepository,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Product {
    id: String,
    name: String,
    price: f64,
    category: String,
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

struct ProductFeeder {
    id: String,
    product: Option<Product>,
}

impl CacheFeed<Product> for ProductFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<Product>) {
        self.product = entity;
    }
}

struct ProductRepository;

impl DataRepository<Product> for ProductRepository {
    async fn fetch_by_id(&self, id: &String) -> Result<Option<Product>> {
        println!("  [DB] Loading product: {}", id);

        let product = match id.as_str() {
            "prod_001" => Some(Product {
                id: id.clone(),
                name: "Laptop".to_string(),
                price: 999.99,
                category: "electronics".to_string(),
            }),
            "prod_002" => Some(Product {
                id: id.clone(),
                name: "Coffee Maker".to_string(),
                price: 49.99,
                category: "appliances".to_string(),
            }),
            _ => None,
        };

        Ok(product)
    }
}

/// Custom metrics implementation
struct SimpleMetrics {
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    misses: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl SimpleMetrics {
    fn new() -> Self {
        SimpleMetrics {
            hits: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            misses: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    #[allow(dead_code)]
    fn stats(&self) -> (usize, usize) {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        (hits, misses)
    }
}

impl CacheMetrics for SimpleMetrics {
    fn record_hit(&self, key: &str, duration: Duration) {
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        println!("    [METRIC] Cache HIT: {} ({:?})", key, duration);
    }

    fn record_miss(&self, key: &str, duration: Duration) {
        self.misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        println!("    [METRIC] Cache MISS: {} ({:?})", key, duration);
    }

    fn record_error(&self, key: &str, error: &str) {
        println!("    [METRIC] ERROR: {}: {}", key, error);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();

    println!("\n=== Cache Kit - Advanced Builder Pattern ===\n");

    // 1. Create TTL policy (per-type)
    println!("1. Setting up per-type TTL policy...");
    let ttl_policy = TtlPolicy::PerType(|entity_type| match entity_type {
        "product" => {
            println!("    → Product TTL: 1 hour");
            Duration::from_secs(3600)
        }
        _ => {
            println!("    → Default TTL: 30 minutes");
            Duration::from_secs(1800)
        }
    });

    // 2. Create expander with configuration
    println!("2. Creating cache expander with custom configuration...\n");
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend)
        .with_metrics(Box::new(SimpleMetrics::new()))
        .with_ttl_policy(ttl_policy);

    let repository = ProductRepository;

    // 3. Execute cache operations with metrics
    println!("3. Executing cache operations:\n");

    // First request - cache miss
    println!("   Request 1: prod_001 (cache miss)");
    let mut feeder = ProductFeeder {
        id: "prod_001".to_string(),
        product: None,
    };
    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(product) = &feeder.product {
        println!("    ✓ Loaded: {} - ${:.2}", product.name, product.price);
    }

    println!();

    // Second request - cache hit
    println!("   Request 2: prod_001 (cache hit)");
    let mut feeder = ProductFeeder {
        id: "prod_001".to_string(),
        product: None,
    };
    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(product) = &feeder.product {
        println!(
            "    ✓ Loaded from cache: {} - ${:.2}",
            product.name, product.price
        );
    }

    println!();

    // Third request - different product
    println!("   Request 3: prod_002 (cache miss)");
    let mut feeder = ProductFeeder {
        id: "prod_002".to_string(),
        product: None,
    };
    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(product) = &feeder.product {
        println!("    ✓ Loaded: {} - ${:.2}", product.name, product.price);
    }

    println!();

    // Invalidate strategy
    println!("   Request 4: prod_001 with Invalidate strategy");
    let mut feeder = ProductFeeder {
        id: "prod_001".to_string(),
        product: None,
    };
    expander
        .with(&mut feeder, &repository, CacheStrategy::Invalidate)
        .await?;

    if let Some(product) = &feeder.product {
        println!(
            "    ✓ Refreshed from database: {} - ${:.2}",
            product.name, product.price
        );
    }

    println!("\n4. Cache statistics:");
    println!("   ✓ Framework successfully uses advanced features:");
    println!("     - Custom metrics tracking");
    println!("     - Per-type TTL policies");
    println!("     - Multiple cache strategies");

    // ========================================================================
    // PER-OPERATION CONFIGURATION: Override TTL for specific operations
    // ========================================================================
    println!("\n5. Per-Operation Configuration Examples:\n");
    println!("   ℹ️  The configurations above (TTL policy, metrics) are set at");
    println!("      setup time and apply to all operations. For per-operation");
    println!("      overrides, use OperationConfig or the operation builder:\n");

    // Example 5a: Short-lived cache for flash sale products
    println!("   Request 5a: Flash sale product with 1-minute TTL override");
    let mut feeder = ProductFeeder {
        id: "prod_001".to_string(),
        product: None,
    };

    // Method 1: Explicit OperationConfig
    let config = cache_kit::OperationConfig::default().with_ttl(Duration::from_secs(60)); // Override to 1 minute instead of 1 hour

    expander
        .with_config(&mut feeder, &repository, CacheStrategy::Refresh, config)
        .await?;

    if let Some(product) = &feeder.product {
        println!(
            "    ✓ Cached with 1-minute TTL: {} - ${:.2}",
            product.name, product.price
        );
    }

    println!();

    // Example 5b: Critical operation with retry logic
    println!("   Request 5b: Critical product lookup with retry");
    let mut feeder = ProductFeeder {
        id: "prod_002".to_string(),
        product: None,
    };

    // Method 2: OperationConfig with both TTL and retry
    let config = cache_kit::OperationConfig::default()
        .with_ttl(Duration::from_secs(300)) // 5-minute TTL override
        .with_retry(3); // Retry up to 3 times on failure

    expander
        .with_config(&mut feeder, &repository, CacheStrategy::Refresh, config)
        .await?;

    if let Some(product) = &feeder.product {
        println!(
            "    ✓ Loaded with retry protection: {} - ${:.2}",
            product.name, product.price
        );
    }

    println!();

    // Example 5c: Compare setup-time vs per-operation config
    println!("   ℹ️  Configuration Levels:\n");
    println!("      Setup-time config (applies to all operations):");
    println!("        - .with_metrics()      → Observability");
    println!("        - .with_ttl_policy()   → Default TTL per entity type");
    println!();
    println!("      Per-operation config (overrides for specific calls):");
    println!("        - .with_config()       → Explicit OperationConfig");
    println!("          └─ .with_ttl()       → Override TTL for this call");
    println!("          └─ .with_retry()     → Add retry logic for this call");

    println!("\n=== Example Complete ===\n");

    Ok(())
}
