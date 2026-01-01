---
layout: single
title: Installation for Agents
description: "Machine-readable installation and setup guide for AI agents and automated tools"
permalink: /installation-agents/
nav_order: 3
date: 2025-12-22
---

---

This page provides structured, machine-readable information for AI agents, code generators, and automated tools integrating cache-kit.

## Quick Reference for Agents

**Essential imports:**

```rust
use cache_kit::{
    CacheService, CacheEntity, CacheFeed, DataRepository,
    backend::InMemoryBackend,  // or RedisBackend, MemcachedBackend
    strategy::CacheStrategy,
    Result,
};
```

**Three required trait implementations:**

1. `CacheEntity` - defines cache key and prefix
2. `DataRepository<T>` - fetches from database
3. `CacheFeed<T>` - receives cached entity

**Basic usage pattern:**

```rust
let cache = CacheService::new(InMemoryBackend::new());
cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;
```

**Available strategies:** `Fresh`, `Refresh` (default), `Invalidate`, `Bypass`

**Backends:** `InMemoryBackend` (dev), `RedisBackend` (prod), `MemcachedBackend` (prod)

## Dependency Declaration

### Minimal Setup (InMemory - Development/Testing)

```toml
[dependencies]
cache-kit = { version = "0.9" }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.41", features = ["rt", "sync", "macros"] }
```

### Production Setup (Redis)

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["redis"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.41", features = ["rt", "sync", "macros"] }
```

### All Backends

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["all"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.41", features = ["rt", "sync", "macros"] }
```

---

## Required Trait Implementations

Every cache-kit integration requires implementing three traits:

### 1. CacheEntity

```rust
pub trait CacheEntity {
    type Key: Into<String>;
    fn cache_key(&self) -> Self::Key;
    fn cache_prefix() -> &'static str;
}
```

**Implementation example:**

```rust
use cache_kit::CacheEntity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
}

impl CacheEntity for User {
    type Key = String;
    fn cache_key(&self) -> Self::Key { self.id.clone() }
    fn cache_prefix() -> &'static str { "user" }
}
```

### 2. DataRepository

```rust
pub trait DataRepository<T: CacheEntity> {
    async fn fetch_by_id(&self, id: &T::Key) -> Result<Option<T>>;
}
```

**Implementation example:**

```rust
use cache_kit::{DataRepository, Result};

struct UserRepository;

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> Result<Option<User>> {
        // Your database fetch logic here
        Ok(Some(User {
            id: id.clone(),
            name: "Alice".to_string(),
        }))
    }
}
```

### 3. CacheFeed

```rust
pub trait CacheFeed<T: CacheEntity> {
    fn entity_id(&mut self) -> T::Key;
    fn feed(&mut self, entity: Option<T>);
}
```

**Implementation example:**

```rust
use cache_kit::CacheFeed;

struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String { self.id.clone() }
    fn feed(&mut self, entity: Option<User>) { self.user = entity; }
}
```

**Alternative: Use GenericFeeder (simpler for basic cases):**

```rust
use cache_kit::feed::GenericFeeder;

// GenericFeeder already implements CacheFeed
let mut feeder = GenericFeeder::<User>::new("user_123".to_string());
// After cache.execute(), access via feeder.data
```

---

## Complete Working Example

**Copy-paste ready example:**

```rust
use cache_kit::{
    CacheService, CacheEntity, CacheFeed, DataRepository,
    backend::InMemoryBackend,
    strategy::CacheStrategy,
    Result,
};
use serde::{Deserialize, Serialize};

// 1. Define entity
#[derive(Clone, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
}

// 2. Implement CacheEntity
impl CacheEntity for User {
    type Key = String;
    fn cache_key(&self) -> Self::Key { self.id.clone() }
    fn cache_prefix() -> &'static str { "user" }
}

// 3. Implement DataRepository
struct UserRepository;

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> Result<Option<User>> {
        // Your database fetch logic here
        Ok(Some(User {
            id: id.clone(),
            name: "Alice".to_string(),
        }))
    }
}

// 4. Implement CacheFeed
struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String { self.id.clone() }
    fn feed(&mut self, entity: Option<User>) { self.user = entity; }
}

// 5. Use it
#[tokio::main]
async fn main() -> Result<()> {
    // Create backend
    let backend = InMemoryBackend::new();

    // Create service
    let cache = CacheService::new(backend);

    // Create repository
    let repository = UserRepository;

    // Create feeder
    let mut feeder = UserFeeder {
        id: "user_123".to_string(),
        user: None,
    };

    // Execute cache operation
    cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;

    // Use the result
    if let Some(user) = feeder.user {
        println!("User: {}", user.name);
    }

    Ok(())
}
```

---

## Initialization Pattern

```rust
use cache_kit::{
    CacheService, CacheEntity, CacheFeed, DataRepository,
    backend::InMemoryBackend,
    strategy::CacheStrategy,
    Result,
};

// Step 1: Create backend
let backend = InMemoryBackend::new();
// OR: RedisBackend::new(config).await?
// OR: MemcachedBackend::new(config).await?

// Step 2: Create service
let cache = CacheService::new(backend);

// Step 3: Execute cache operation (must be in async context)
cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;
```

---

## Backend Configuration Reference

### InMemory Backend

- **Feature:** `inmemory` (enabled by default)
- **Use Case:** Development, testing, single-instance services
- **Initialization:** `InMemoryBackend::new()` (no config required)
- **Thread-safe:** Yes (uses DashMap internally)
- **External dependencies:** None

### Redis Backend

- **Feature:** `redis`
- **Use Case:** Production distributed caching
- **Initialization:** `RedisBackend::new(config).await?`

**Configuration struct:**

```rust
RedisConfig {
    host: String,                      // Default: "localhost"
    port: u16,                         // Default: 6379
    username: Option<String>,          // Default: None
    password: Option<String>,          // Default: None
    database: u32,                     // Default: 0
    pool_size: u32,                    // Default: 16
    connection_timeout: Duration,      // Default: 5 seconds
}
```

**Configuration Examples:**

```rust
use cache_kit::backend::RedisConfig;
use std::time::Duration;

// Basic (localhost)
let config = RedisConfig::default();

// With authentication
let config = RedisConfig {
    host: "redis.example.com".to_string(),
    port: 6379,
    password: Some("secret".to_string()),
    ..Default::default()
};

// Custom database
let config = RedisConfig {
    host: "localhost".to_string(),
    port: 6379,
    database: 1,
    ..Default::default()
};
```

**Environment variable pattern:**

```rust
use cache_kit::backend::{RedisBackend, RedisConfig};
use std::time::Duration;

let redis_host = std::env::var("REDIS_HOST")
    .unwrap_or_else(|_| "localhost".to_string());
let redis_password = std::env::var("REDIS_PASSWORD").ok();
let config = RedisConfig {
    host: redis_host,
    password: redis_password,
    ..Default::default()
};
let backend = RedisBackend::new(config).await?;
```

### Memcached Backend

- **Feature:** `memcached`
- **Use Case:** High-performance distributed caching
- **Initialization:** `MemcachedBackend::new(config).await?`

**Configuration struct:**

```rust
use cache_kit::backend::MemcachedConfig;
use std::time::Duration;

MemcachedConfig {
    servers: Vec<String>,              // Required: Memcached server addresses
    connection_timeout: Duration,     // Default: 5 seconds
    pool_size: u32,                   // Default: 16
}
```

**Configuration Examples:**

```rust
use cache_kit::backend::{MemcachedBackend, MemcachedConfig};
use std::time::Duration;

// Basic (single server)
let config = MemcachedConfig {
    servers: vec!["localhost:11211".to_string()],
    ..Default::default()
};
let backend = MemcachedBackend::new(config).await?;

// Multiple servers (uses first server for connection pool)
let config = MemcachedConfig {
    servers: vec![
        "localhost:11211".to_string(),
        "cache-2.example.com:11211".to_string(),
    ],
    pool_size: 20,
    connection_timeout: Duration::from_secs(10),
};
let backend = MemcachedBackend::new(config).await?;
```

---

## Key Types and Methods

| Type               | Constructor/Method                               | Purpose                      |
| ------------------ | ------------------------------------------------ | ---------------------------- |
| `CacheService<B>`  | `new(backend)`                                   | Main cache orchestration     |
| `CacheService<B>`  | `execute(&mut feeder, &repo, strategy)`          | Execute cache operation      |
| `CacheStrategy`    | `Fresh` \| `Refresh` \| `Invalidate` \| `Bypass` | Cache execution strategy     |
| `InMemoryBackend`  | `new()`                                          | Development backend          |
| `RedisBackend`     | `new(config).await?`                             | Production Redis backend     |
| `MemcachedBackend` | `new(config).await?`                             | Production Memcached backend |

---

## Common Integration Patterns

### Pattern A: Basic Entity Caching

1. Define entity struct with `#[derive(Serialize, Deserialize)]`
2. Implement `CacheEntity` - define cache prefix and key
3. Implement `DataRepository` - fetch from database
4. Implement `CacheFeed` - store fetched entity
5. Call `cache.execute()` in service layer
6. Handle `Result` error cases

### Pattern B: Dependency Injection

```rust
use cache_kit::{CacheService, backend::InMemoryBackend};

// Create once at startup
let cache = CacheService::new(InMemoryBackend::new());

// Pass to services
let user_service = UserService::new(cache.clone());
let order_service = OrderService::new(cache.clone());

// Note: CacheService implements Clone (cheap Arc increment)
```

### Pattern C: Environment-Based Backend Selection

```rust
use cache_kit::{
    CacheService,
    backend::{InMemoryBackend, RedisBackend, RedisConfig},
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cache = match std::env::var("ENV").as_deref() {
        Ok("production") => {
            let redis_host = std::env::var("REDIS_HOST")
                .unwrap_or_else(|_| "localhost".to_string());
            let redis_password = std::env::var("REDIS_PASSWORD").ok();

            let config = RedisConfig {
                host: redis_host,
                password: redis_password,
                pool_size: 20,
                ..Default::default()
            };
            CacheService::new(RedisBackend::new(config).await?)
        }
        _ => CacheService::new(InMemoryBackend::new())
    };

    // Use cache...
    Ok(())
}
```

**Note:** For type-erased backends across different types, consider using an enum wrapper or trait objects. For most cases, the above pattern works when you know the backend type at compile time.

---

## Serialization Support

**Supported formats:**

- **Postcard** (Tier-1, recommended for performance)
- **Custom implementations** via trait impl

**Known Limitations:**

- Some types not supported by Postcard (e.g., `Decimal`)

**Workarounds:**

- Convert `Decimal` to `i64` cents
- Implement custom serialization wrapper
- Use alternative serialization format

---

## Error Handling

Cache-kit returns `Result<T, Error>`. Always handle:

- Backend connection failures
- Serialization errors
- Repository fetch failures
- Network timeouts (Redis/Memcached)

**Best practice:** Application should gracefully degrade if cache is unavailable.

```rust
use cache_kit::{Result, Error};

match cache.execute(&mut feeder, &repository, strategy).await {
    Ok(_) => {
        // Success - use feeder.entity
        if let Some(user) = feeder.user {
            // Process user
        }
    },
    Err(e) => {
        // Log error
        eprintln!("Cache error: {}", e);
        // Fall back to direct repository fetch
        feeder.user = repository.fetch_by_id(&feeder.id).await.ok().flatten();
    }
}
```

---

## Cache Strategies

| Strategy     | Behavior                                | Use Case                         |
| ------------ | --------------------------------------- | -------------------------------- |
| `Refresh`    | Try cache first, fallback to DB on miss | Default, balanced approach       |
| `Fresh`      | Use cache only, return None on miss     | When data must be cached         |
| `Invalidate` | Clear cache, fetch fresh from DB        | After mutations, need fresh data |
| `Bypass`     | Skip cache entirely, always use DB      | Testing or temporary disable     |

---

## Deployment Configuration Checklist

- [ ] Backend selected: InMemory (dev) or Redis/Memcached (prod)
- [ ] Connection pooling configured for expected load
- [ ] Serialization format validated for all entity types
- [ ] Error handlers implemented for all cache calls
- [ ] Environment variables set (`REDIS_URL`, backend addresses)
- [ ] Fallback strategy defined if cache unavailable
- [ ] TTL policies defined (if using expiration)
- [ ] Load testing completed
- [ ] Monitoring/metrics enabled for cache hit/miss rates

---

## Reference Links

- [Installation (Human)](/cache-kit.rs/installation/) - Detailed human-readable guide
- [Core Concepts](/cache-kit.rs/concepts/) - Conceptual deep-dive
- [Database Compatibility](/cache-kit.rs/database-compatibility/) - Supported ORMs and query builders
- [Cache Backends](/cache-kit.rs/backends/) - Backend detailed reference
- [Serialization](/cache-kit.rs/serialization/) - Serialization format and limitations
- [Async Model](/cache-kit.rs/async-model/) - Async/await patterns
