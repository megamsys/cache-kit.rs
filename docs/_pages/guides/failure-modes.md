---
layout: single
title: Failure Modes & Error Handling
parent: Guides
nav_order: 11
date: 2025-12-30
---

## Understanding cache-kit Error Behavior

cache-kit is designed to be **resilient by default** but requires you to understand failure modes for production deployments.

This guide answers the critical question: **"What happens when Redis dies mid-request?"**

---

## Scenario 1: Cache Backend Failure (Redis/Memcached Down)

### Behavior by Strategy

| Strategy       | Cache Fails              | Database Available | Result                      |
| -------------- | ------------------------ | ------------------ | --------------------------- |
| **Fresh**      | Return Error             | N/A (no DB call)   | `Error::BackendError`       |
| **Refresh**    | Fall back to DB          | Fetch from DB      | ✅ Success (logged warning) |
| **Invalidate** | Try delete, ignore error | Fetch from DB      | ✅ Success                  |
| **Bypass**     | Ignore cache             | Fetch from DB      | ✅ Success                  |

**Key Insight:** `CacheStrategy::Refresh` provides automatic degradation to database-only mode when cache fails.

### Example

```rust
use cache_kit::{CacheService, CacheStrategy};

// Redis is down, but database is available
match cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await {
    Ok(_) => {
        // Success! Data fetched from database (cache backend error logged)
        println!("User: {:?}", feeder.user);
    }
    Err(e) => {
        // Only fails if BOTH cache AND database are unreachable
        eprintln!("Total failure: {}", e);
    }
}
```

### Detailed Strategy Behavior

#### Fresh Strategy

```rust
// Attempt cache-only fetch
cache.execute(&mut feeder, &repo, CacheStrategy::Fresh).await
```

**When cache backend fails:**

- Returns `Error::BackendError` immediately
- **Does NOT** fall back to database
- Use case: Assume data is cached; cache miss is an error condition

#### Refresh Strategy (Default)

```rust
// Try cache first, fallback to database
cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await
```

**When cache backend fails:**

1. Logs warning about cache backend unavailability
2. Proceeds to fetch from database
3. Attempts to cache the result (may silently fail if backend still down)
4. Returns success with data from database

**Production benefit:** Your application keeps running even when Redis is down.

#### Invalidate Strategy

```rust
// Clear cache and refresh
cache.execute(&mut feeder, &repo, CacheStrategy::Invalidate).await
```

**When cache backend fails:**

1. Attempts to delete cache entry
2. Ignores deletion error (best-effort invalidation)
3. Fetches fresh data from database
4. Attempts to cache result
5. Returns success if database fetch succeeds

#### Bypass Strategy

```rust
// Skip cache entirely
cache.execute(&mut feeder, &repo, CacheStrategy::Bypass).await
```

**When cache backend fails:**

- Ignores cache completely (by design)
- Fetches from database
- Attempts to cache result for other requests
- Returns success if database succeeds

---

## Scenario 2: Database Failure (Cache Available)

### Behavior by Strategy

| Strategy       | Cache Hit     | Database Fails   | Result                   |
| -------------- | ------------- | ---------------- | ------------------------ |
| **Fresh**      | Return cached | N/A (no DB call) | ✅ Success               |
| **Refresh**    | Return cached | N/A (cache hit)  | ✅ Success               |
| **Refresh**    | Cache miss    | Return Error     | `Error::RepositoryError` |
| **Invalidate** | Delete cache  | Return Error     | `Error::RepositoryError` |
| **Bypass**     | Ignore cache  | Return Error     | `Error::RepositoryError` |

**Key Insight:** Cached data shields your application from database outages.

### Example

```rust
// Database is down, but cache has data
match cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await {
    Ok(_) if feeder.user.is_some() => {
        // Success! Served from cache (database error logged but hidden)
        println!("User from cache: {:?}", feeder.user);
    }
    Err(cache_kit::Error::RepositoryError(e)) => {
        // Cache miss + database down = total failure
        eprintln!("Database unreachable and cache miss: {}", e);
    }
    _ => {}
}
```

---

## Scenario 3: Total Failure (Both Down)

When both cache backend AND database are unreachable:

### Behavior

```rust
match cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await {
    Err(cache_kit::Error::RepositoryError(e)) => {
        // Both systems failed
        // Return 503 Service Unavailable to client
    }
    _ => {}
}
```

**Application responsibilities:**

1. Return HTTP 503 (Service Unavailable)
2. Implement circuit breaker to stop hammering failing services
3. Serve static fallback content if possible
4. Alert on-call team

---

## Production Recommendations

### 1. Use CacheStrategy::Refresh by Default

Provides best resilience - automatically falls back to database when cache fails.

```rust
// Recommended default
cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await?
```

**Fallback chain:**

1. Try cache
2. On cache failure → Try database
3. On database failure → Return error

## Error Type Reference

### Error::BackendError

**Cause:** Cache backend (Redis/Memcached) is unreachable or returned error

**Common triggers:**

- Redis connection lost
- Network timeout
- Redis out of memory
- Authentication failure

**Recovery:**

- Retry with exponential backoff
- Fall back to database (automatic with `Refresh` strategy)
- Serve stale cache if available

### Error::RepositoryError

**Cause:** Database fetch failed

**Common triggers:**

- Database connection lost
- Query timeout
- Row/record not found
- SQL syntax error

**Recovery:**

- Serve cached data if available
- Retry with backoff
- Return 503 to client if critical

### Error::SerializationError

**Cause:** Entity could not be serialized for caching

**Common triggers:**

- Non-serializable type in entity
- Serde macro error
- Postcard encoding failure

**Recovery:**

- Log error and skip caching
- Return data from database without caching
- Fix entity definition

### Error::DeserializationError

**Cause:** Cached data is corrupted or incompatible

**Common triggers:**

- Schema version changed
- Cache corrupted during write
- Non-cache-kit data in Redis key

**Recovery:**

- Invalidate cache entry
- Fetch fresh from database
- Log for investigation

### Error::VersionMismatch

**Cause:** Schema version in cache doesn't match code

**Common triggers:**

- Code deployment with schema changes
- Struct fields added/removed
- Enum variants changed

**Recovery:**

- **Automatic:** cache-kit invalidates entry and refetches
- **No action needed** - expected during deployments

---

## Summary

| Failure           | Fresh    | Refresh   | Invalidate | Bypass    |
| ----------------- | -------- | --------- | ---------- | --------- |
| Cache down, DB up | ❌ Error | ✅ Use DB | ✅ Use DB  | ✅ Use DB |
| Cache up, DB down | ✅ Cache | ✅ Cache  | ❌ Error   | ❌ Error  |
| Both down         | ❌ Error | ❌ Error  | ❌ Error   | ❌ Error  |
| Both up           | ✅ Cache | ✅ Cache  | ✅ DB      | ✅ DB     |

**Takeaway:** `CacheStrategy::Refresh` is the most resilient default choice for production.
