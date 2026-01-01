---
layout: single
title: Installation & Configuration
description: "Getting started with cache-kit in your Rust project"
permalink: /installation/
nav_order: 2
date: 2025-12-21
---

---

## Prerequisites

- **Rust:** 1.75 or later
- **Tokio:** 1.41 or later (async runtime)

---

## For AI Agents & Automated Tools

If you're using an AI agent, code generator, or automated tool to integrate cache-kit, see the [Installation for Agents](/cache-kit.rs/installation-agents/) page for structured, machine-readable setup information.

---

## Installation

Add cache-kit to your `Cargo.toml`:

```toml
[dependencies]
cache-kit = { version = "0.9" }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.41", features = ["rt", "sync", "macros"] }
```

### Feature Flags

cache-kit uses feature flags to enable optional backends:

| Feature     | Description                           | Default     |
| ----------- | ------------------------------------- | ----------- |
| `inmemory`  | In-memory cache backend               | ✅ Enabled  |
| `redis`     | Redis backend with connection pooling | ❌ Optional |
| `memcached` | Memcached backend                     | ❌ Optional |
| `all`       | Enable all backends                   | ❌ Optional |

---

## Choosing Your API: CacheExpander vs CacheService

cache-kit provides two APIs for cache operations. **For most use cases, use `CacheService`.**

### Quick Decision Table

| Scenario                           | Use              | Reason                        |
| ---------------------------------- | ---------------- | ----------------------------- |
| Building web service (Axum, Actix) | **CacheService** | Already Arc'd, clone is cheap |
| Building a library                 | CacheExpander    | Flexibility in Arc wrapping   |
| Uncertain                          | **CacheService** | 90% of use cases              |

> **⚠️ Can't decide? Use CacheService.** It's simpler and covers most cases.

> **📖 For deeper understanding:** See [Core Concepts](/cache-kit.rs/concepts/) for detailed explanations of `CacheExpander` and `CacheService`, including design philosophy, usage patterns, and examples throughout the documentation.

### CacheService (Recommended for Web Applications)

**Use CacheService when:**

- Building web services (Axum, Actix, Rocket)
- Need to share cache across threads
- Want simple, ergonomic API

**Key characteristics:**

- Already wrapped in `Arc` internally
- Implements `Clone` (cheap reference counting)
- Methods: `.execute()`, `.execute_with_config()`
- Perfect for dependency injection

**Example:**

```rust
use cache_kit::{CacheService, backend::RedisBackend};

// Create once at startup
let cache = CacheService::new(backend);

// Share across services (Clone is cheap - just Arc increment)
let user_service = UserService::new(cache.clone());
let product_service = ProductService::new(cache.clone());
let order_service = OrderService::new(cache.clone());
```

### CacheExpander (Low-Level API)

**Use CacheExpander when:**

- Need custom Arc wrapping patterns
- Building cache middleware or custom abstractions
- Want explicit control over ownership

**Key characteristics:**

- No built-in Arc wrapper
- Methods: `.with()`, `.with_config()`
- Requires manual `Arc` wrapping for sharing

**Example:**

```rust
use cache_kit::{CacheExpander, backend::RedisBackend};
use std::sync::Arc;

let expander = CacheExpander::new(backend);

// Must wrap in Arc manually for sharing
let cache = Arc::new(expander);
let user_service = UserService::new(cache.clone());
```

---

### Basic Installation (InMemory Only)

```toml
[dependencies]
cache-kit = { version = "0.9" }
```

This provides the InMemory backend, perfect for:

- Development
- Testing
- Single-instance services

### Redis Backend

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["redis"] }
```

Enables production-grade Redis caching with connection pooling.

### Memcached Backend

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["memcached"] }
```

Enables Memcached backend for high-performance distributed caching.

### All Backends

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["all"] }
```

Enables all available backends. Useful for:

- Testing multiple backends
- Switching backends based on environment
- Benchmarking comparisons

---

## Minimal Configuration

cache-kit requires minimal configuration. Here's a complete working example:

```rust
use cache_kit::{
    CacheEntity, CacheFeed, DataRepository, CacheService,
    backend::InMemoryBackend,
    strategy::CacheStrategy,
};
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

struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String { self.id.clone() }
    fn feed(&mut self, entity: Option<User>) { self.user = entity; }
}

struct UserRepository;

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<User>> {
        Ok(Some(User {
            id: id.clone(),
            name: "Alice".to_string(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = InMemoryBackend::new();
    let cache = CacheService::new(backend);
    let repository = UserRepository;

    let mut feeder = UserFeeder {
        id: "user_001".to_string(),
        user: None,
    };

    cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;

    println!("User: {:?}", feeder.user);
    Ok(())
}
```

---

## Backend Configuration

### ⚠️ Production Backend Requirement

**InMemory backend is for development/testing only.** Use Redis or Memcached for production deployments.

---

### InMemory Backend

No configuration required:

```rust
use cache_kit::{CacheService, backend::InMemoryBackend};

let cache = CacheService::new(InMemoryBackend::new());
```

The InMemory backend uses `DashMap` internally, providing:

- Lock-free concurrent HashMap
- Thread-safe operations
- Zero external dependencies

### Redis Backend

```rust
use cache_kit::{CacheService, backend::{RedisBackend, RedisConfig}};
use std::time::Duration;

let config = RedisConfig {
    host: "localhost".to_string(),
    port: 6379,
    username: None,
    password: None,
    database: 0,
    pool_size: 16,
    connection_timeout: Duration::from_secs(5),
};

let backend = RedisBackend::new(config).await?;
let cache = CacheService::new(backend);
```

#### Redis Configuration Options

| Field                | Type             | Default       | Description                       |
| -------------------- | ---------------- | ------------- | --------------------------------- |
| `host`               | `String`         | `"localhost"` | Redis server hostname or IP       |
| `port`               | `u16`            | `6379`        | Redis server port                 |
| `username`           | `Option<String>` | `None`        | Redis username (Redis 6+)         |
| `password`           | `Option<String>` | `None`        | Redis password for authentication |
| `database`           | `u32`            | `0`           | Redis database number (0-15)      |
| `pool_size`          | `u32`            | `16`          | Connection pool size              |
| `connection_timeout` | `Duration`       | `5s`          | Connection timeout                |

#### Configuration Examples

```rust
use std::time::Duration;

// Basic configuration (all defaults)
let config = RedisConfig::default();

// Custom host and port
let config = RedisConfig {
    host: "redis.example.com".to_string(),
    port: 6379,
    ..Default::default()
};

// With authentication
let config = RedisConfig {
    host: "redis.example.com".to_string(),
    port: 6379,
    password: Some("secret".to_string()),
    database: 1,
    ..Default::default()
};

// With custom pool size
let config = RedisConfig {
    host: "localhost".to_string(),
    port: 6379,
    pool_size: 32,
    connection_timeout: Duration::from_secs(10),
    ..Default::default()
};
```

#### Environment-Based Configuration

```rust
use std::env;
use std::time::Duration;

let redis_host = env::var("REDIS_HOST")
    .unwrap_or_else(|_| "localhost".to_string());
let redis_port = env::var("REDIS_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(6379);
let redis_password = env::var("REDIS_PASSWORD").ok();

let config = RedisConfig {
    host: redis_host,
    port: redis_port,
    password: redis_password,
    ..Default::default()
};

let backend = RedisBackend::new(config).await?;
```

### Memcached Backend

```rust
use cache_kit::{CacheService, backend::{MemcachedBackend, MemcachedConfig}};
use std::time::Duration;

let config = MemcachedConfig {
    servers: vec!["localhost:11211".to_string()],
    pool_size: 16,
    connection_timeout: Duration::from_secs(5),
};

let cache = CacheService::new(MemcachedBackend::new(config).await?);
```

#### Memcached Configuration Options

| Field                | Type          | Default  | Description                |
| -------------------- | ------------- | -------- | -------------------------- |
| `servers`            | `Vec<String>` | Required | Memcached server addresses |
| `pool_size`          | `u32`         | `16`     | Connection pool size       |
| `connection_timeout` | `Duration`    | `5s`     | Connection timeout         |

#### Multiple Memcached Servers

```rust
use cache_kit::backend::{MemcachedBackend, MemcachedConfig};
use std::time::Duration;

let config = MemcachedConfig {
    servers: vec![
        "memcached-01:11211".to_string(),
        "memcached-02:11211".to_string(),
        "memcached-03:11211".to_string(),
    ],
    pool_size: 20,
    connection_timeout: Duration::from_secs(10),
};

let backend = MemcachedBackend::new(config).await?;
```

---

## TTL Configuration

Configure time-to-live (TTL) for cached entries:

### Global TTL

```rust
use std::time::Duration;
use cache_kit::{CacheService, observability::TtlPolicy, backend::InMemoryBackend};

let cache = CacheService::new(InMemoryBackend::new());
// Note: TTL configuration via CacheService is set through backend configuration
```

### No TTL (Cache Forever)

```rust
use cache_kit::{CacheService, backend::InMemoryBackend};

// Don't set TTL - cached entries never expire
let cache = CacheService::new(InMemoryBackend::new());
```

**Note:** "Cache forever" is not recommended for production. Always set appropriate TTLs based on your data freshness requirements.

---

## Environment-Based Configuration

Create a configuration module for your application:

```rust
use cache_kit::{CacheService, backend::{InMemoryBackend, RedisBackend, RedisConfig}};
use std::env;
use std::time::Duration;

pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn from_env() -> Self {
        match env::var("ENV").as_deref() {
            Ok("production") => Environment::Production,
            _ => Environment::Development,
        }
    }
}

pub async fn create_cache_service() -> Result<CacheService<impl cache_kit::backend::CacheBackend>, Box<dyn std::error::Error>> {
    match Environment::from_env() {
        Environment::Development => {
            Ok(CacheService::new(InMemoryBackend::new()))
        }
        Environment::Production => {
            let redis_host = env::var("REDIS_HOST")
                .unwrap_or_else(|_| "localhost".to_string());
            let redis_password = env::var("REDIS_PASSWORD").ok();

            let config = RedisConfig {
                host: redis_host,
                password: redis_password,
                pool_size: 20,
                connection_timeout: Duration::from_secs(10),
                ..Default::default()
            };

            let backend = RedisBackend::new(config).await?;
            Ok(CacheService::new(backend))
        }
    }
}
```

Usage:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache = create_cache_service().await?;

    // Your application logic
    Ok(())
}
```

---

## Docker Compose for Development

Use Docker Compose to run Redis and Memcached locally:

```yaml
version: "3.8"

services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes

  memcached:
    image: memcached:1.6-alpine
    ports:
      - "11211:11211"
    command: memcached -m 64

volumes:
  redis_data:
```

Start services:

```bash
docker-compose up -d
```

Test connections:

```bash
# Redis
redis-cli ping  # Should return: PONG

# Memcached
echo "stats" | nc localhost 11211  # Should return stats
```

---

## Testing Configuration

For unit and integration tests, use the InMemory backend:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use cache_kit::backend::InMemoryBackend;

    #[tokio::test]
    async fn test_user_caching() {
        // Use InMemory backend for tests (no external dependencies)
        let backend = InMemoryBackend::new();
        let mut expander = CacheExpander::new(backend);

        // Your test logic
    }
}
```

---

## Production Checklist

Before deploying cache-kit to production:

- [ ] **Backend selected:** Redis or Memcached for production
- [ ] **Connection pooling configured:** Set `pool_size` in config (default: 16) or via `REDIS_POOL_SIZE`/`MEMCACHED_POOL_SIZE` environment variables
- [ ] **TTL policies defined:** Set TTLs based on data freshness requirements
- [ ] **Error handling implemented:** Handle cache failures gracefully
- [ ] **Monitoring enabled:** Track cache hit/miss rates
- [ ] **Environment variables set:** `REDIS_HOST`, `REDIS_PORT`, `REDIS_PASSWORD`, `REDIS_POOL_SIZE` (for Redis) or `MEMCACHED_SERVERS`, `MEMCACHED_POOL_SIZE` (for Memcached)
- [ ] **Fallback strategy:** Application works if cache is unavailable
- [ ] **Load tested:** Verify performance under expected load

---

## Common Configuration Patterns

### Pattern 1: Shared Cache Across Services

```rust
use cache_kit::{CacheService, backend::{RedisBackend, RedisConfig}};

let backend = RedisBackend::new(config).await?;
let cache = CacheService::new(backend);

// CacheService is Clone - easily share across services
let user_service = UserService::new(cache.clone());
let product_service = ProductService::new(cache.clone());
```

---

## Next Steps

- Learn about [Database & ORM compatibility](/cache-kit.rs/database-compatibility/)
- Explore [Cache backend options](/cache-kit.rs/backends/) in detail
- Review [Serialization formats](/cache-kit.rs/serialization/)
- See the [Actix + SQLx example](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)
