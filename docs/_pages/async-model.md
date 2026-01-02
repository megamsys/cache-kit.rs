---
layout: single
title: Async Programming Model
description: "Understanding cache-kit's async-first design and tokio integration"
permalink: /async-model/
nav_order: 6
date: 2025-12-25
---

---

## Async-First Philosophy

cache-kit is built from the ground up as an **async-first** library. This design choice reflects the reality of modern Rust services where:

- Database queries are async (SQLx, SeaORM, tokio-postgres)
- HTTP handlers are async (Axum, Actix, warp)
- gRPC services are async (tonic)
- Background workers are async (tokio, async-std)

The cache layer sits between these components and must integrate seamlessly with async workflows.

---

## Tokio Runtime Integration

cache-kit is designed for `tokio`-based applications. The library does not:

- Spawn its own runtime
- Require a specific runtime configuration
- Impose threading models on your application

Instead, cache-kit operates within **your** existing tokio runtime.

### Runtime Requirements

```toml
[dependencies]
tokio = { version = "1.41", features = ["rt", "sync", "macros"] }
cache-kit = "0.9"
```

The minimum required tokio features:

- `rt` — Runtime support
- `sync` — Synchronization primitives (Arc, Mutex, RwLock)
- `macros` — `#[tokio::main]` attribute macro

---

## Interaction Model

The typical interaction flow follows this pattern:

```
Async Database → Async Cache → Async Application
```

All cache operations work seamlessly within async contexts. For detailed ORM integration examples (SQLx, SeaORM, Diesel), see [Database & ORM Compatibility](/cache-kit.rs/database-compatibility).

---

## Why DataRepository is Async

The `DataRepository` trait uses **async** methods:

```rust
pub trait DataRepository<T: CacheEntity>: Send + Sync {
    async fn fetch_by_id(&self, id: &T::Key) -> Result<Option<T>>;
}
```

This design is intentional and provides several benefits:

1. **Native async support** — Aligns with modern Rust practices and integrates seamlessly with async databases
2. **Flexibility** — Works with both sync and async database layers (see [Database Compatibility](/cache-kit.rs/database-compatibility) for Diesel example using `spawn_blocking`)
3. **Backend compatibility** — Cache backends (Redis, Memcached) are inherently async

**Recommended async databases:**

- **SQLx** — Async, compile-time checked SQL
- **SeaORM** — Async ORM for Rust
- **tokio-postgres** — Pure async PostgreSQL client

For detailed repository implementation examples, see [Database & ORM Compatibility](/cache-kit.rs/database-compatibility).

### ⚠️ NEVER use block_in_place + block_on

**NEVER use `block_in_place` + `Handle::current().block_on()`** — this pattern is incorrect. Always use `async fn` with `.await` for async databases. For synchronous ORMs like Diesel, use `tokio::task::spawn_blocking` (see [Database Compatibility](/cache-kit.rs/database-compatibility) for examples).

---

## Async Cache Backends

Cache backends are fully async and follow the same initialization pattern:

```rust
// Redis
use cache_kit::backend::{RedisBackend, RedisConfig};
let config = RedisConfig::default(); // Uses localhost:6379
let backend = RedisBackend::new(config).await?;

// Memcached
use cache_kit::backend::{MemcachedBackend, MemcachedConfig};
let config = MemcachedConfig { servers: vec!["localhost:11211".to_string()], ..Default::default() };
let backend = MemcachedBackend::new(config)?;

// InMemory (lock-free via DashMap)
use cache_kit::backend::InMemoryBackend;
let backend = InMemoryBackend::new();

// All use the same expander API
let mut expander = CacheExpander::new(backend);
```

All backends work seamlessly within your async context — no special handling required.

---

## Runtime Choice is Yours

cache-kit does not:

- Require a specific tokio runtime configuration
- Spawn background tasks (no `tokio::spawn` calls)
- Create thread pools
- Impose executor choices

You control:

- Runtime flavor (multi-thread, current-thread)
- Worker thread count
- Task spawning strategy
- Shutdown behavior

---

## Best Practices

### DO

- ✅ Use `tokio::main` for your application entry point
- ✅ Make `DataRepository::fetch_by_id` an async function
- ✅ Use async database drivers (SQLx, SeaORM, tokio-postgres)
- ✅ Let cache-kit operate within your existing runtime
- ✅ Keep async boundaries explicit and clear

### DON'T

- ❌ Use `block_in_place` + `block_on` (incorrect pattern)
- ❌ Call `block_on` inside async contexts
- ❌ Create multiple tokio runtimes unnecessarily
- ❌ Assume cache-kit manages runtime lifecycle

---

## Example: Full Async Service

For complete working examples of tokio-based services using cache-kit with async operations:

- **[examples/actixsqlx](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)** — Actix Web integration with SQLx, async database operations, and cache-kit's async API
- **[examples/actixsqlx/src/services/user_service.rs](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx/src/services/user_service.rs)** — Service layer implementation with caching

---

## Next Steps

- Learn about [Core Concepts](/cache-kit.rs/concepts) in cache-kit
- Explore [Database & ORM Compatibility](/cache-kit.rs/database-compatibility) for detailed ORM integration examples
- Review [API Frameworks](/cache-kit.rs/api-frameworks) for framework-specific integration patterns
