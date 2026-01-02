# cache-kit

**A type-safe, generic caching framework for Rust.**

[![Crates.io](https://img.shields.io/crates/v/cache-kit?style=flat-square)](https://crates.io/crates/cache-kit)
[![Docs.rs](https://img.shields.io/docsrs/cache-kit?style=flat-square)](https://docs.rs/cache-kit)
[![Documentation](https://img.shields.io/badge/guide-cachekit.org-blue?style=flat-square)](http://cachekit.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![CI](https://github.com/megamsys/cache-kit.rs/actions/workflows/ci.yml/badge.svg)](https://github.com/megamsys/cache-kit.rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/megamsys/cache-kit.rs/branch/main/graph/badge.svg)](https://codecov.io/gh/megamsys/cache-kit.rs)

A trait-based caching framework that works with any Rust type, any backend (InMemory, Redis, Memcached), and any database (SQLx, Diesel, tokio-postgres).

---

## Features

- **Generic** — Cache any type `T` that implements `CacheEntity`
- **Backend Agnostic** — Switch between InMemory, Redis, Memcached without code changes
- **Database Agnostic** — Works with any repository implementing `DataRepository`
- **Type Safe** — Compile-time verified
- **Thread Safe** — `Send + Sync` guarantees

---

## ⚠️ Requirements

- Requires `tokio` runtime (async-only)
- Use Redis/Memcached for production (InMemory is single-instance only)
- Decimal types (`rust_decimal::Decimal`) require custom serialization

For production guidance, see the [documentation](http://cachekit.org/).

## Quick Start

### Add to Cargo.toml

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["inmemory"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use cache_kit::{
    CacheEntity, CacheFeed, DataRepository, CacheExpander,
    backend::InMemoryBackend,
    strategy::CacheStrategy,
};
use serde::{Deserialize, Serialize};

// 1. Define your entity
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

// 3. Create a feeder
struct UserFeeder {
    id: String,
    user: Option<User>,
}

impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String { self.id.clone() }
    fn feed(&mut self, entity: Option<User>) { self.user = entity; }
}

// 4. Create a repository
struct UserRepository;

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<User>> {
        // Fetch from your database
        Ok(Some(User { id: id.clone(), name: "Alice".to_string() }))
    }
}

// 5. Use the cache
#[tokio::main]
async fn main() -> cache_kit::Result<()> {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend);

    let mut feeder = UserFeeder { id: "user_001".to_string(), user: None };
    let repository = UserRepository;

    expander.with(&mut feeder, &repository, CacheStrategy::Refresh).await?;

    if let Some(user) = feeder.user {
        println!("User: {}", user.name);
    }

    Ok(())
}
```

## Cache Strategies

| Strategy              | Behavior                   | Use Case                     |
| --------------------- | -------------------------- | ---------------------------- |
| **Refresh** (default) | Try cache, fallback to DB  | Most common case             |
| **Fresh**             | Cache only, no DB fallback | When data must be cached     |
| **Invalidate**        | Clear cache, fetch from DB | After mutations              |
| **Bypass**            | Skip cache, always use DB  | Testing or temporary disable |

## Advanced Usage

For complex operations like custom TTL overrides, retry logic, and builder patterns, see the [documentation](http://cachekit.org/) for advanced features including metrics, TTL policies, and `CacheService` for service-oriented architectures.

## Backends

### In-Memory (Default)

```rust
use cache_kit::backend::InMemoryBackend;
let backend = InMemoryBackend::new();
let expander = CacheExpander::new(backend);
```

### Redis

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["redis"] }
```

```rust
use cache_kit::backend::{RedisBackend, RedisConfig};
let backend = RedisBackend::new(RedisConfig::default())?;
let expander = CacheExpander::new(backend);
```

### Memcached

```toml
[dependencies]
cache-kit = { version = "0.9", features = ["memcached"] }
```

```rust
use cache_kit::backend::{MemcachedBackend, MemcachedConfig};
let backend = MemcachedBackend::new(MemcachedConfig::default())?;
let expander = CacheExpander::new(backend);
```

## Observability

The framework supports logging (via `log` crate) and custom metrics:

```rust
use cache_kit::observability::{CacheMetrics, TtlPolicy};
use std::time::Duration;

// Custom metrics
let expander = CacheExpander::new(backend)
    .with_metrics(Box::new(MyMetrics));

// TTL policies
let expander = CacheExpander::new(backend)
    .with_ttl_policy(TtlPolicy::Fixed(Duration::from_secs(300)));
```

See the [documentation](http://cachekit.org/) for details on metrics and TTL policies.

## Examples

```bash
# Basic usage
cargo run --example basic_usage

# Multiple backends
cargo run --example multiple_backends --features redis,memcached

# Advanced builder
cargo run --example advanced_builder
```

## Testing

```bash
make up
make test FEATURES="--all-features"
```

For more examples and guides, see the [documentation](http://cachekit.org/).

## Publishing

```bash
make up
make release
```

This runs build, test, audit, and publish. For linting, use `make dev`.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - Copyright (c) 2025 Kishore kumar Neelamegam

See [LICENSE](LICENSE) file for details.

## Support

**Maintainer:** Kishore kumar Neelamegam
**Repository:** [github.com/megamsys/cache-kit.rs](https://github.com/megamsys/cache-kit.rs)
**Issues:** [GitHub Issues](https://github.com/megamsys/cache-kit.rs/issues)
**Crates.io:** [crates.io/crates/cache-kit](https://crates.io/crates/cache-kit)
**Documentation:** [cachekit.org](https://cachekit.org)
