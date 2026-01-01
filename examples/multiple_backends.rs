//! Example demonstrating backend swapping.

use cache_kit::{
    backend::{CacheBackend, InMemoryBackend},
    error::Result,
    strategy::CacheStrategy,
    CacheEntity, CacheExpander, CacheFeed, DataRepository,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
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

struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<User>) {
        self.user = entity;
    }
}

struct UserRepository;

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> Result<Option<User>> {
        Ok(Some(User {
            id: id.clone(),
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        }))
    }
}

/// Generic function that works with any cache backend
async fn demonstrate_cache<B: CacheBackend>(backend: B, backend_name: &str) -> Result<()> {
    println!("\n--- Using {} Backend ---", backend_name);

    let expander = CacheExpander::new(backend);
    let repository = UserRepository;

    // First request - cache miss
    println!("Request 1: Fetching user_001");
    let mut feeder = UserFeeder {
        id: "user_001".to_string(),
        user: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(user) = &feeder.user {
        println!("  ✓ Got: {} ({})", user.name, user.email);
    }

    // Second request - cache hit
    println!("Request 2: Fetching user_001 (should be cached)");
    let mut feeder = UserFeeder {
        id: "user_001".to_string(),
        user: None,
    };

    expander
        .with(&mut feeder, &repository, CacheStrategy::Refresh)
        .await?;

    if let Some(user) = &feeder.user {
        println!("  ✓ Got from cache: {} ({})", user.name, user.email);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .ok();

    println!("\n=== Cache Kit - Multiple Backends ===");

    // In-Memory Backend (always available)
    {
        let backend = InMemoryBackend::new();
        demonstrate_cache(backend, "InMemory").await?;
    }

    // Redis Backend (if feature enabled)
    #[cfg(feature = "redis")]
    {
        use cache_kit::backend::{RedisBackend, RedisConfig};

        match RedisBackend::new(RedisConfig::default()).await {
            Ok(backend) => {
                demonstrate_cache(backend, "Redis").await?;
            }
            Err(e) => {
                println!(
                    "\n✗ Redis backend unavailable: {} (Make sure Redis is running)",
                    e
                );
            }
        }
    }

    // Memcached Backend (if feature enabled)
    #[cfg(feature = "memcached")]
    {
        use cache_kit::backend::{MemcachedBackend, MemcachedConfig};

        match MemcachedBackend::new(MemcachedConfig::default()).await {
            Ok(backend) => {
                demonstrate_cache(backend, "Memcached").await?;
            }
            Err(e) => {
                println!(
                    "\n✗ Memcached backend unavailable: {} (Make sure Memcached is running)",
                    e
                );
            }
        }
    }

    println!("\n=== Example Complete ===\n");
    println!("Note: Same code works with any backend!");
    println!("      Just swap the backend implementation.\n");

    Ok(())
}
