//! Basic usage example of the cache framework.

use cache_kit::{
    backend::InMemoryBackend, error::Result, strategy::CacheStrategy, CacheEntity, CacheFeed,
    DataRepository,
};
use serde::{Deserialize, Serialize};

/// Example entity: Employment
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Employment {
    id: String,
    loanapp_id: String,
    employer_name: String,
    salary: f64,
    hire_date: String,
}

impl CacheEntity for Employment {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn cache_prefix() -> &'static str {
        "employment"
    }
}

/// Feeder for Employment
struct EmploymentFeeder {
    id: String,
    employment: Option<Employment>,
}

impl CacheFeed<Employment> for EmploymentFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<Employment>) {
        self.employment = entity;
    }
}

/// Mock repository that simulates database access
struct EmploymentRepository;

impl DataRepository<Employment> for EmploymentRepository {
    async fn fetch_by_id(&self, id: &String) -> Result<Option<Employment>> {
        println!("  [DB] Fetching employment: {}", id);

        // Simulate some employments in the database
        let employment = match id.as_str() {
            "emp_001" => Some(Employment {
                id: id.clone(),
                loanapp_id: "loan_123".to_string(),
                employer_name: "Acme Corp".to_string(),
                salary: 75000.0,
                hire_date: "2023-01-15".to_string(),
            }),
            "emp_002" => Some(Employment {
                id: id.clone(),
                loanapp_id: "loan_456".to_string(),
                employer_name: "Tech Inc".to_string(),
                salary: 95000.0,
                hire_date: "2022-06-01".to_string(),
            }),
            _ => None,
        };

        Ok(employment)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init()
        .ok();

    println!("\n=== Cache Kit - Basic Example ===\n");

    // 1. Initialize cache backend
    println!("1. Initializing in-memory cache backend...");
    let backend = InMemoryBackend::new();
    let expander = cache_kit::CacheExpander::new(backend);
    let repository = EmploymentRepository;

    println!("   ✓ Cache backend ready\n");

    // 2. First request - cache miss, fetch from database
    println!("2. First request for employment (emp_001):");
    let mut feeder = EmploymentFeeder {
        id: "emp_001".to_string(),
        employment: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(emp) = &feeder.employment {
        println!(
            "   ✓ Employment loaded: {} from {} (${:.2})\n",
            emp.employer_name, emp.id, emp.salary
        );
    }

    // 3. Second request - cache hit
    println!("3. Second request for same employment (emp_001):");
    let mut feeder = EmploymentFeeder {
        id: "emp_001".to_string(),
        employment: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(emp) = &feeder.employment {
        println!(
            "   ✓ Employment loaded from cache: {} (${:.2})\n",
            emp.employer_name, emp.salary
        );
    }

    // 4. Fresh strategy - cache only
    println!("4. Fresh strategy (cache only):");
    let mut feeder = EmploymentFeeder {
        id: "emp_003".to_string(), // Not in cache or database
        employment: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Fresh)
        .await?;

    if feeder.employment.is_none() {
        println!("   ✓ Cache miss, no database fallback (as expected)\n");
    }

    // 5. Invalidate strategy - force refresh
    println!("5. Invalidate strategy (force refresh):");
    let mut feeder = EmploymentFeeder {
        id: "emp_002".to_string(),
        employment: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Invalidate)
        .await?;

    if let Some(emp) = &feeder.employment {
        println!(
            "   ✓ Employment refreshed from database: {} (${:.2})\n",
            emp.employer_name, emp.salary
        );
    }

    // 6. Bypass strategy - skip cache
    println!("6. Bypass strategy (skip cache):");
    let mut feeder = EmploymentFeeder {
        id: "emp_001".to_string(),
        employment: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Bypass)
        .await?;

    if let Some(emp) = &feeder.employment {
        println!(
            "   ✓ Employment fetched directly from database: {} (${:.2})\n",
            emp.employer_name, emp.salary
        );
    }

    // ========================================================================
    // ADVANCED USAGE: Per-Operation Configuration
    // ========================================================================
    // For most use cases, the simple .with() method above is sufficient.
    // However, if you need to override TTL or add retry logic for specific
    // operations, you can use one of the advanced patterns below:

    println!("\n=== Advanced Usage Examples ===\n");

    // 7. Advanced: Explicit config with TTL override and retry
    println!("7. Advanced: Using OperationConfig for TTL override:");
    let mut feeder = EmploymentFeeder {
        id: "emp_001".to_string(),
        employment: None,
    };

    // Create a config with custom TTL and retry logic
    let config = cache_kit::OperationConfig::default()
        .with_ttl(std::time::Duration::from_secs(300)) // Cache for 5 minutes instead of default
        .with_retry(3); // Retry up to 3 times on failure

    expander
        .with_config(&mut feeder, &repository, CacheStrategy::Refresh, config)
        .await?;

    if let Some(emp) = &feeder.employment {
        println!(
            "   ✓ Employment loaded with custom TTL: {} (${:.2})\n",
            emp.employer_name, emp.salary
        );
    }

    // 8. Advanced: Fluent builder pattern (same as above, but more ergonomic)
    println!("8. Advanced: Using OperationConfig for TTL override and retry:");
    let mut feeder = EmploymentFeeder {
        id: "emp_002".to_string(),
        employment: None,
    };

    let config = cache_kit::OperationConfig::default()
        .with_ttl(std::time::Duration::from_secs(300))
        .with_retry(3);

    expander
        .with_config(&mut feeder, &repository, CacheStrategy::Refresh, config)
        .await?;

    if let Some(emp) = &feeder.employment {
        println!(
            "   ✓ Employment loaded with config: {} (${:.2})\n",
            emp.employer_name, emp.salary
        );
    }

    println!("=== Example Complete ===\n");

    println!("\nℹ️  Note: OperationConfig is useful when you need per-operation");
    println!("   TTL overrides or retry logic. For 90% of use cases,");
    println!("   the simple .with() method is recommended.\n");

    Ok(())
}
