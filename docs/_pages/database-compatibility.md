---
layout: single
title: Database & ORM Compatibility
description: "Using cache-kit with different ORMs and database layers"
permalink: /database-compatibility/
nav_order: 8
date: 2025-12-27
---

---

## ORM-Agnostic Design

cache-kit does **not depend on ORMs**.

It operates on three simple concepts:

- **Serializable entities** — Any type implementing `CacheEntity`
- **Deterministic cache keys** — Consistent identifiers
- **Explicit cache boundaries** — Clear separation via `CacheFeed`

This means:

- ✅ Swap ORMs without changing cache logic
- ✅ Use multiple ORMs in the same application
- ✅ Cache data from any source (DB, API, file system)

---

## Supported ORMs & Database Layers

### Tier-1: Recommended (with Examples)

| ORM      | Status          | Example                                                                            | Notes                                 |
| -------- | --------------- | ---------------------------------------------------------------------------------- | ------------------------------------- |
| **SQLx** | ✅ Full Support | [actixsqlx](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx) | Async-first, compile-time checked SQL |

### Tier-1: Compatible (Community Examples Welcome)

| ORM                | Status        | Example                         | Notes                        |
| ------------------ | ------------- | ------------------------------- | ---------------------------- |
| **SeaORM**         | ✅ Compatible | Community contributions welcome | Async ORM with migrations    |
| **Diesel**         | ✅ Compatible | Community contributions welcome | Mature, type-safe ORM        |
| **tokio-postgres** | ✅ Compatible | Works with any database layer   | Pure async PostgreSQL client |

### Tier-2: Any Database Layer

cache-kit works with **any** Rust code that can:

1. Fetch entities by ID
2. Return `Option<T>` (entity or not found)
3. Implement `DataRepository<T>` trait

This includes:

- Custom SQL builders
- NoSQL databases (MongoDB, DynamoDB)
- REST API clients
- File-based storage
- In-memory data structures

---

## Conceptual Flow

```
Application Code
    ↓
┌─────────────────────┐
│ cache-kit           │ ← Coordinator (framework-agnostic)
└──────────┬──────────┘
           │
           ├─→ Check cache first
           │   ↓
           │ ┌─────────────────────┐
           │ │ Cache Backend       │ ← Redis, Memcached, InMemory
           │ └─────────────────────┘
           │
           └─→ If cache miss, fetch from repository
               ↓
           ┌─────────────────────┐
           │ DataRepository      │ ← impl DataRepository<T>
           └──────────┬──────────┘
                      │
                      ↓ Fetch by ID
           ┌─────────────────────┐
           │ Database / ORM      │ ← Your choice (SQLx, SeaORM, etc.)
           └──────────┬──────────┘
                      │
                      ↓ Returns
           ┌─────────────────────┐
           │ Domain Entities     │ ← impl CacheEntity
           └─────────────────────┘
                      │
                      ↑ (stored in cache)
```

**Key principle:** cache-kit coordinates between cache and database. It checks the cache first, and only queries the database (via `DataRepository`) on cache misses. Domain entities implement `CacheEntity` and are stored in the cache backend.

---

## SQLx Integration

SQLx is an async, compile-time checked SQL library. It's the recommended database layer for new projects.

### Installation

```toml
[dependencies]
cache-kit = { version = "0.9" }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### Entity Definition

```rust
use serde::{Deserialize, Serialize};
use cache_kit::CacheEntity;

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
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
```

### Repository Implementation

```rust
use cache_kit::DataRepository;
use sqlx::PgPool;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user: &User) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, username, email)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            user.id,
            user.username,
            user.email
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(&self, user: &User) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET username = $2, email = $3
            WHERE id = $1
            RETURNING *
            "#,
            user.id,
            user.username,
            user.email
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| cache_kit::Error::RepositoryError(e.to_string()))?;

        Ok(user)
    }
}
```

### Usage in Service Layer

```rust
use cache_kit::{CacheService, CacheFeed, DataRepository, strategy::CacheStrategy};
use cache_kit::backend::InMemoryBackend;
use std::sync::Arc;

pub struct UserService {
    cache: CacheService<InMemoryBackend>,
    repo: Arc<UserRepository>,
}

impl UserService {
    pub async fn get_user(&self, id: &str) -> cache_kit::Result<Option<User>> {
        let mut feeder = UserFeeder {
            id: id.to_string(),
            user: None,
        };

        self.cache
            .execute(&mut feeder, &*self.repo, CacheStrategy::Refresh)
            .await?;

        Ok(feeder.user)
    }
}
```

**Complete examples:**

- **[examples/actixsqlx](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)** — Full Actix + SQLx implementation
- **[examples/actixsqlx/src/services/user_service.rs](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx/src/services/user_service.rs)** — Service layer with caching

---

## Database Best Practices

### Separate Concerns

```
✅ Good:
    Database Models → Repository → Cache → Service → API

❌ Bad:
    Database Models with embedded cache logic
```

### Repository Pattern

Keep repositories focused on data access:

```rust
impl UserRepository {
    // ✅ Simple, focused data access
    pub async fn find_by_id(&self, id: &str) -> Result<Option<User>, DbError> {
        sqlx::query_as!(...).await
    }

    // ❌ Don't mix cache logic in repository
    pub async fn find_by_id_cached(&self, id: &str) -> Result<Option<User>, DbError> {
        // BAD: Repository shouldn't know about caching
    }
}
```

### Error Handling

Convert database errors to cache-kit errors:

```rust
impl DataRepository<User> for UserRepository {
    fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<User>> {
        self.internal_fetch(id)
            .map_err(|e| cache_kit::Error::RepositoryError(e.to_string()))
    }
}
```

---

## Database Migrations

cache-kit does not handle database migrations. Use your ORM's migration tools:

### SQLx

For SQLx migration setup and usage, please refer to the [SQLx documentation](https://github.com/launchbadge/sqlx).

---

## Next Steps

- Learn about [Core Concepts](/cache-kit.rs/concepts) — Understanding cache-kit fundamentals
- Review [Async Programming Model](/cache-kit.rs/async-model) — Async-first design
- Explore [API Frameworks](/cache-kit.rs/api-frameworks) — Framework integration examples
- See [Serialization options](/cache-kit.rs/serialization) — Postcard and serialization
- Review [Cache backend choices](/cache-kit.rs/backends) — Redis, Memcached, InMemory
