---
layout: single
title: "Quick Start: Request Lifecycle"
description: "Understanding how cache-kit handles HTTP requests from start to finish"
permalink: /guides/quick-start-request-lifecycle/
nav_order: 4
date: 2025-12-23
---

## Overview

This guide walks you through a complete HTTP request lifecycle with cache-kit, from the moment an HTTP request arrives to when the response is returned. You'll see exactly how cache hits and misses work, when the database is queried, and how the cache automatically populates.

Understanding this flow is essential for effective cache-kit usage in production web services.

### What You'll Learn

- How cache-kit integrates with HTTP handlers
- The exact sequence of operations for cache hits vs misses
- Timing expectations for cached vs database queries
- When and how errors are handled
- How to implement this pattern in your own service

---

## Typical Request Flow

Here's what happens when a client requests user data:

```
Client → HTTP GET /users/123 → Handler → cache.execute() → Response
```

The `cache.execute()` call is where the magic happens. Let's break it down step-by-step.

---

## Step-by-Step Walkthrough

### Step 1: HTTP Request Arrives

```rust
#[get("/users/{id}")]
async fn get_user(
    id: web::Path<String>,
    cache: web::Data<CacheService<RedisBackend>>,
    repo: web::Data<UserRepository>,
) -> Result<Json<User>> {
    // Request starts here
```

The handler receives:

- `id`: The user ID from the URL path (`"123"`)
- `cache`: Shared cache service instance
- `repo`: Database repository for fetching users

### Step 2: Create Feeder with Entity ID

```rust
    let mut feeder = UserFeeder {
        id: id.into_inner(),
        user: None,  // Will be populated by cache or DB
    };
```

The `UserFeeder` holds:

- `id`: The identifier we want to fetch
- `user`: Initially `None`, will be filled by `cache.execute()`

### Step 3: Execute Cache Operation

```rust
    cache
        .execute(&mut feeder, &*repo, CacheStrategy::Refresh)
        .await?;
```

This is the core operation. Here's what happens inside:

```
execute() ──→ Does Redis have "user:123"?
              ├─ YES (Cache Hit) → Deserialize → feed() → feeder.user = Some(user)
              │                     [~1-2ms total]
              │
              └─ NO (Cache Miss) → Query database
                                    Serialize with Postcard
                                    Store in Redis (TTL=300s)
                                    feed() → feeder.user = Some(user)
                                    [~10-100ms total]
```

### Step 4: Extract Result

```rust
    match feeder.user {
        Some(user) => Ok(Json(user)),
        None => Err(Error::NotFound),
    }
}
```

After `execute()` completes, `feeder.user` contains the result:

- `Some(user)`: User was found (from cache or DB)
- `None`: User doesn't exist in the system

---

## Cache Hit Path

**What happens:** Redis has the cached entry for `"user:123"`.

**Timing:** ~1-2ms (Redis network latency + deserialization)

**Code flow:**

1. `cache.execute()` checks Redis for key `"user:123"`
2. Redis returns serialized bytes
3. cache-kit deserializes using Postcard
4. Calls `feeder.feed(Some(user))` to populate `feeder.user`
5. Returns to handler

**When this happens:**

- Second and subsequent requests for the same user ID
- Within the TTL window (default: 5 minutes)
- If cache hasn't been invalidated

**Performance benefit:**

- Database is NOT queried
- Response time: 1-2ms vs 10-100ms (50x-100x faster)
- Reduced database load

---

## Cache Miss Path

**What happens:** Redis does NOT have the cached entry for `"user:123"`.

**Timing:** ~10-100ms (database query + serialization + Redis store)

**Code flow:**

1. `cache.execute()` checks Redis for key `"user:123"` → NOT FOUND
2. Calls `repo.fetch_by_id("123")` to query the database
3. Database returns `User` struct
4. cache-kit serializes the user with Postcard
5. Stores in Redis with key `"user:123"` and TTL
6. Calls `feeder.feed(Some(user))` to populate `feeder.user`
7. Returns to handler

**When this happens:**

- First request for a user ID
- TTL has expired (e.g., 5 minutes passed)
- Cache was manually invalidated
- Cache service was restarted (if using InMemory backend)

**Automatic caching:**

The new entry is **automatically cached** for subsequent requests. No manual cache population needed!

---

## Error Handling

cache-kit uses the `Refresh` strategy by default, which provides robust error handling:

### Scenario 1: Redis is Down

**What happens:**

- `cache.execute()` attempts to connect to Redis → FAILS
- Falls back to database query automatically
- Application continues to work (degraded performance)

**Code behavior:**

```rust
// No code changes needed - Refresh strategy handles this
cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await?;
```

**Impact:**

- All requests hit the database (no caching)
- Response time: 10-100ms (same as cache miss)
- Database load increases

### Scenario 2: Database is Down

**What happens:**

- `cache.execute()` checks Redis → HIT
- Returns cached data without touching the database
- Application continues to work normally

**Impact:**

- Only cached requests succeed
- Requests for new/expired data will fail
- Cache acts as a **protective shield** for read operations

### Scenario 3: Both Redis and Database are Down

**What happens:**

- `cache.execute()` returns an error
- Handler returns HTTP 500 or error response

**Mitigation:**

For production resilience, see [Failure Modes & Resilience](/cache-kit.rs/guides/failure-modes/) for:

- Circuit breaker patterns
- Graceful degradation strategies
- Fallback mechanisms

---

## Complete Example

Here's a full working example you can run:

```rust
use actix_web::{web, get, App, HttpServer, Result};
use cache_kit::{
    CacheService, CacheEntity, CacheFeed, DataRepository,
    backend::RedisBackend,
    strategy::CacheStrategy,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
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
        // Simulate database query
        Ok(Some(User {
            id: id.clone(),
            name: "Alice".to_string(),
        }))
    }
}

#[get("/users/{id}")]
async fn get_user(
    id: web::Path<String>,
    cache: web::Data<CacheService<RedisBackend>>,
    repo: web::Data<UserRepository>,
) -> Result<web::Json<User>> {
    let mut feeder = UserFeeder {
        id: id.into_inner(),
        user: None,
    };

    cache
        .execute(&mut feeder, &*repo, CacheStrategy::Refresh)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    match feeder.user {
        Some(user) => Ok(web::Json(user)),
        None => Err(actix_web::error::ErrorNotFound("User not found")),
    }
}
```

**Test it:**

```bash
# First request (cache miss, ~10-100ms)
curl http://localhost:8080/users/123

# Second request (cache hit, ~1-2ms)
curl http://localhost:8080/users/123
```

---

## Next Steps

Now that you understand the request lifecycle, explore:

- [Core Concepts](/cache-kit.rs/concepts/) — Deep dive into feeders, entities, and cache strategies
- [Actix + SQLx Example](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx) — Production-ready implementation with real database
- [Failure Modes & Resilience](/cache-kit.rs/guides/failure-modes/) — Handle cache and database failures gracefully
- [Monitoring & Metrics](/cache-kit.rs/guides/monitoring/) — Track cache hit rates and latency in production
