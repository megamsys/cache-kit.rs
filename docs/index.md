---
layout: single
title: Introduction
description: "cache-kit is an async, ORM-agnostic caching library for Rust services"
nav_order: 1
date: 2025-12-20
---

# cache-kit

An async, ORM-agnostic caching library for Rust that helps you place **clear cache boundaries** between your database and application logic.

[Get Started](installation){: .btn .btn-primary }
[View on GitHub](https://github.com/megamsys/cache-kit.rs){: .btn }

---

## What cache-kit Is

cache-kit is designed for modern Rust services that:

- Use async runtimes (tokio)
- Talk to databases through ORMs or query builders
- Expose functionality via REST, gRPC, background workers, or API services

cache-kit focuses on **how caching fits into your system**, not on owning your framework, transport, or persistence layer.

### Key Characteristics

- **Async-first and runtime-friendly** — Built for tokio-based applications
- **ORM-agnostic** — Works with any database layer
- **Backend-agnostic** — Swap between Redis, Memcached, InMemory without code changes
- **Safe to embed** — Use inside libraries, SDKs, and services
- **Explicit boundaries** — Clear cache ownership and behavior

cache-kit sits **between your database access and your application logic**, not inside your HTTP framework or ORM.

---

## What cache-kit Is Not

cache-kit deliberately does **not**:

- Replace ORMs or query builders
- Depend on HTTP, REST, or web frameworks
- Hide cache behavior behind implicit magic
- Claim strong consistency across distributed backends
- Act as a full application framework

If you are looking for an all-in-one web stack, cache-kit is not that.

---

## Where cache-kit Fits

A typical async Rust service using cache-kit looks like this:

```
REST / gRPC / Workers / API Services
      ↓
Application Logic
      ↓
  cache-kit
      ↙        ↘
Cache Backend   Database / ORM
```

The same cached logic can be reused across:

- REST endpoints
- gRPC services
- Background jobs
- Agent or API services layered on top of your data

---

## Async-First by Design

cache-kit is built for async Rust:

- Designed to work with `tokio`-based applications
- Compatible with async ORMs like SQLx and SeaORM
- Does not manage or impose a runtime

You bring your runtime — cache-kit fits into it.

**Note:** cache-kit is async-first and designed for modern async databases (SQLx, SeaORM, tokio-postgres). See the [Async Programming Model](async-model) page for details.

---

## ORM-Agnostic and Database-Friendly

cache-kit does **not depend on any ORM**.

It operates on:

- Serializable entities
- Deterministic cache keys
- Explicit cache boundaries

This means:

- You can swap ORMs without rewriting cache logic
- Cache behavior lives outside persistence concerns
- Database models and cached entities remain your responsibility

### ORM Compatibility

| ORM/Database Layer | Status        | Example                                                                                    |
| ------------------ | ------------- | ------------------------------------------------------------------------------------------ |
| **SQLx**           | ✅ Tier-1     | [actixsqlx example](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx) |
| **SeaORM**         | ✅ Compatible | Community contributions welcome                                                            |
| **Diesel**         | ✅ Compatible | Community contributions welcome                                                            |
| **tokio-postgres** | ✅ Compatible | Works with any database layer                                                              |

A reference implementation using **Actix + SQLx** is provided, but cache-kit is not tied to that stack.

---

## Serialization Is Explicit

cache-kit treats serialization as a **first-class, pluggable concern**.

### Supported Formats

- **Postcard** (Tier-1, recommended for performance)
- **MessagePack** (Planned, community contributions welcome)

Serialization is:

- Independent of transport (HTTP, gRPC, workers)
- Independent of cache backend (Redis, Memcached, InMemory)
- Chosen explicitly by the user

### Known Limitations

Some serialization formats (including **Postcard**) do **not support certain data types** out of the box.

For example:

- `Decimal` types are **not supported** by Postcard
- You must either:
  - Convert to supported primitives (e.g., store as `i64` cents instead of `Decimal` dollars)
  - Implement custom serialization
  - Choose a different serialization strategy

cache-kit does not silently work around these limitations — they are part of the design trade-off.

See the [Serialization Support](serialization) page for detailed guidance.

---

## Backend-Agnostic Caching

cache-kit supports multiple cache backends with explicit tiering:

### Tier-0: Production-Grade

- **Redis** and Redis-compatible managed services (AWS ElastiCache, DigitalOcean Managed Redis)
- **Memcached**

### Tier-1: Development & Testing

- **InMemory** — Fast, zero-dependency, perfect for tests and local development

Backends are treated as **replaceable implementations**, not architectural commitments.

See the [Cache Backend Support](backends) page for configuration details.

---

## Design Philosophy

cache-kit focuses on:

- **Boundaries, not ownership** — Integrate around ORMs, frameworks, and transports
- **Explicit behavior, not hidden magic** — No surprises, predictable outcomes
- **Integration, not lock-in** — Works with your existing stack

It is safe to use:

- Inside libraries
- Inside SDKs
- Inside large services
- Alongside existing frameworks and ORMs

cache-kit aims to be predictable, composable, and honest about trade-offs.

---

## Quick Example

```rust
use cache_kit::{
    CacheEntity, CacheFeed, DataRepository, CacheService,
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
        // Your database fetch logic
        Ok(Some(User {
            id: id.clone(),
            name: "Alice".to_string(),
        }))
    }
}

// 5. Use the cache
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = InMemoryBackend::new();
    let cache = CacheService::new(backend);

    let mut feeder = UserFeeder {
        id: "user_001".to_string(),
        user: None,
    };
    let repository = UserRepository;

    cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;

    if let Some(user) = feeder.user {
        println!("User: {}", user.name);
    }

    Ok(())
}
```

---

## Next Steps

- Learn the [core concepts](concepts) behind cache-kit
- Understand [async usage patterns](async-model)
- Explore [ORM and backend compatibility](/cache-kit.rs/database-compatibility)
- Review the [Actix + SQLx reference implementation](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)

---

## License

MIT License - Copyright (c) 2025 Kishore kumar Neelamegam

See [LICENSE](https://github.com/megamsys/cache-kit.rs/blob/main/LICENSE) for details.
