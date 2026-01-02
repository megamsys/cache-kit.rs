---
layout: single
title: Concepts
description: "Understanding the fundamental concepts behind cache-kit"
permalink: /concepts/
nav_order: 5
date: 2025-12-24
---

cache-kit is built around four core concepts that work together to provide clean, explicit caching boundaries:

1. **Serializable Entities** — Type-safe data models
2. **Deterministic Cache Keys** — Consistent, predictable addressing
3. **Explicit Cache Boundaries** — Clear ownership and behavior
4. **Cache Invalidation Control** — You decide when data becomes stale

These concepts are **intentionally simple** and avoid framework-specific abstractions.

---

## Serializable Entities

An entity in cache-kit is any Rust type that can be:

1. **Serialized** to bytes (for storage in cache)
2. **Deserialized** from bytes (for retrieval from cache)
3. **Cloned** (for internal cache operations)
4. **Identified** by a unique key

### The CacheEntity Trait

```rust
use cache_kit::CacheEntity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
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
```

### What Makes an Entity Cacheable?

| Requirement      | Purpose                                     |
| ---------------- | ------------------------------------------- |
| `Clone`          | Cache operations need to duplicate entities |
| `Serialize`      | Convert to bytes for storage                |
| `Deserialize`    | Convert from bytes for retrieval            |
| `Send + Sync`    | Safe to share across threads                |
| `cache_key()`    | Unique identifier for this entity           |
| `cache_prefix()` | Namespace for entity type                   |

### Cache Key Construction

The final cache key is constructed as:

```
{prefix}:{key}
```

For the User example above:

```rust
let user = User {
    id: "user_001".to_string(),
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
};

// Final cache key: "user:user_001"
```

This pattern ensures:

- **No collisions** between different entity types
- **Predictable keys** for debugging and monitoring
- **Type safety** at compile time

---

## Deterministic Cache Keys

Cache keys must be **deterministic** — given the same entity, you always get the same key.

### Good Key Examples

```rust
// ✅ Simple ID
impl CacheEntity for User {
    type Key = String;
    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }
}

// ✅ Composite key
impl CacheEntity for OrderItem {
    type Key = String;
    fn cache_key(&self) -> Self::Key {
        format!("{}:{}", self.order_id, self.item_id)
    }
}

// ✅ Numeric ID
impl CacheEntity for Product {
    type Key = u64;
    fn cache_key(&self) -> Self::Key {
        self.product_id
    }
}
```

### Anti-Patterns to Avoid

```rust
// ❌ Non-deterministic (timestamp)
fn cache_key(&self) -> String {
    format!("{}:{}", self.id, SystemTime::now().timestamp())
}

// ❌ Non-deterministic (random)
fn cache_key(&self) -> String {
    format!("{}:{}", self.id, rand::random::<u64>())
}

// ❌ Overly complex (hash collisions possible)
fn cache_key(&self) -> String {
    format!("{:x}", calculate_hash(&self))
}
```

**Rule:** Cache keys should depend **only** on stable entity attributes.

---

## Explicit Cache Boundaries

cache-kit uses a **feeder pattern** to define explicit cache boundaries.

### The CacheFeed Trait

A feeder acts as a bridge between cache-kit and your application:

```rust
use cache_kit::CacheFeed;

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
```

### Why Feeders?

Without feeders, the cache would return values directly. This creates problems:

- **Ownership issues** — Returning owned values or references gets complicated with the borrow checker
- **Flexibility loss** — You'd need separate methods for each entity type
- **Repetition** — Every service method would duplicate cache logic manually

Feeders solve this by acting as a **container that holds both the request (ID) and response (entity)**:

1. **Explicit data flow** — You control where cached data goes
2. **Type safety** — Compiler enforces correct usage
3. **No hidden state** — No implicit global caches
4. **Testability** — Easy to mock and verify
5. **Generic operations** — One `execute()` method works for any entity type

### Feeder Lifecycle

```
1. Create feeder with entity ID
        ↓
2. Pass feeder to cache expander
        ↓
3. Cache expander calls entity_id()
        ↓
4. Cache hit → feed() called with entity
   Cache miss → fetch from repository → feed() called
        ↓
5. Application reads entity from feeder
```

### Example: Using a Feeder

```rust
// 1. Create feeder with the ID you want to fetch
let mut feeder = UserFeeder {
    id: "user_001".to_string(),
    user: None,
};

// 2. Execute cache operation (async)
expander.with::<User, _, _>(&mut feeder, &repository, CacheStrategy::Refresh).await?;

// 3. Access the result
if let Some(user) = feeder.user {
    println!("Found user: {}", user.name);
} else {
    println!("User not found");
}
```

---

## Cache Strategies

cache-kit provides four explicit cache strategies:

### 1. Fresh (Cache-Only)

```rust
CacheStrategy::Fresh
```

- **Behavior:** Return entity from cache, or `None` if not cached
- **Use case:** When you ONLY want cached data, never database
- **Example:** Real-time dashboards showing last known state

```rust
cache.execute(&mut feeder, &repository, CacheStrategy::Fresh).await?;

match feeder.user {
    Some(user) => println!("Cached user: {}", user.name),
    None => println!("Not in cache"),
}
```

### 2. Refresh (Cache + Database Fallback)

```rust
CacheStrategy::Refresh
```

- **Behavior:** Try cache first, fallback to database on miss, then cache the result
- **Use case:** **Default and recommended** for most operations
- **Example:** User profile lookups, product details

```rust
cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;

// Will always have data (if it exists in DB)
if let Some(user) = feeder.user {
    println!("User: {}", user.name);
}
```

### 3. Invalidate (Clear + Refresh)

```rust
CacheStrategy::Invalidate
```

- **Behavior:** Remove from cache, fetch from database, cache the fresh result
- **Use case:** After updates/writes to ensure fresh data
- **Example:** After user updates profile

```rust
// User updated their profile (in your service layer)
// ... update logic ...

// Invalidate cache and fetch fresh data
expander.with::<User, _, _>(&mut feeder, &repository, CacheStrategy::Invalidate).await?;
```

### 4. Bypass (Database-First)

```rust
CacheStrategy::Bypass
```

- **Behavior:** Skip cache lookup, always fetch from database first, then populate cache
- **Use case:** One-off queries, debugging, auditing, ensuring absolute freshness
- **Example:** Admin operations that need guaranteed fresh data

```rust
// Always fetch from database first, then cache the result
cache.execute(&mut feeder, &repository, CacheStrategy::Bypass).await?;
```

### Strategy Decision Tree

```
Need data?
  ├─ Only cached? → Fresh
  ├─ Fresh from DB required? → Invalidate or Bypass
  ├─ Normal read? → Refresh (default)
  └─ Debugging? → Bypass
```

---

## Data Repository Pattern

cache-kit is agnostic to your data source. You define how to fetch entities:

### The DataRepository Trait

```rust
use cache_kit::DataRepository;

pub trait DataRepository<T: CacheEntity>: Send + Sync {
    async fn fetch_by_id(&self, id: &T::Key) -> cache_kit::Result<Option<T>>;
}
```

### Example: SQLx Repository

```rust
use sqlx::PgPool;

struct UserRepository {
    pool: PgPool,
}

impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, name, email FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| cache_kit::Error::RepositoryError(e.to_string()))?;

        Ok(user)
    }
}
```

### Example: In-Memory Repository (for Testing)

cache-kit provides `InMemoryRepository` for testing. No need to implement it yourself:

```rust
use cache_kit::repository::InMemoryRepository;

// Create and populate test repository
let mut repo = InMemoryRepository::<User>::new();
repo.insert("user_001".to_string(), user_entity);

// Use with cache operations
cache.execute(&mut feeder, &repo, CacheStrategy::Refresh).await?;
```

### Repository Best Practices

✅ **DO:**

- Keep repositories focused on data fetching only
- Return `Option<T>` to distinguish "not found" from errors
- Use proper error types (convert DB errors to cache-kit errors)
- Make repositories cloneable (`Arc` wrapper)

❌ **DON'T:**

- Put cache logic inside repositories
- Mix business logic with data access
- Assume entities exist (always return Option)
- Panic on database errors

**For ORM-specific repository implementations** (SQLx, SeaORM, Diesel), see [Database & ORM Compatibility](/database-compatibility).

---

## Cache Ownership and Invalidation

You own cache invalidation. cache-kit does not:

- Automatically invalidate on writes
- Track entity relationships
- Provide distributed invalidation
- Guess when data is stale

### Invalidation Patterns

#### Pattern 1: Invalidate After Write

```rust
use cache_kit::{CacheService, CacheStrategy, backend::InMemoryBackend};

pub struct UserService {
    cache: CacheService<InMemoryBackend>,
    repository: UserRepository,
}

impl UserService {
    pub async fn update_user(&self, user: &User) -> cache_kit::Result<()> {
        // 1. Update database (your update logic here)
        // ... update logic ...

        // 2. Invalidate cache and fetch fresh data
        let mut feeder = UserFeeder {
            id: user.id.clone(),
            user: None,
        };
        self.cache.execute::<User, _, _>(
            &mut feeder,
            &self.repository,
            CacheStrategy::Invalidate
        ).await?;

        Ok(())
    }
}
```

#### Pattern 2: TTL-Based Expiry

```rust
use cache_kit::{CacheExpander, observability::TtlPolicy, backend::InMemoryBackend};
use std::time::Duration;

// Option 1: Fixed TTL (same for all entities)
let ttl_policy = TtlPolicy::Fixed(Duration::from_secs(3600)); // 1 hour
let expander = CacheExpander::new(InMemoryBackend::new())
    .with_ttl_policy(ttl_policy);

// Option 2: Per-Type TTL (different for each entity type)
let ttl_policy = TtlPolicy::PerType(|entity_type| {
    match entity_type {
        "user" => Duration::from_secs(3600),        // 1 hour
        "product" => Duration::from_secs(86400),    // 1 day
        _ => Duration::from_secs(1800),             // 30 min default
    }
});

let expander = CacheExpander::new(InMemoryBackend::new())
    .with_ttl_policy(ttl_policy);

// Cache entries expire automatically based on TTL policy
expander.with::<User, _, _>(&mut feeder, &repository, CacheStrategy::Refresh).await?;
```

---

## Configuration Levels: Setup-Time vs Per-Operation

cache-kit provides two configuration levels to balance simplicity with flexibility:

### Setup-Time Configuration (Applied to All Operations)

Setup-time configuration is set once when creating the cache and applies to **all** operations:

```rust
use cache_kit::{CacheExpander, backend::InMemoryBackend, observability::TtlPolicy};
use std::time::Duration;

// Configure at setup time
let expander = CacheExpander::new(InMemoryBackend::new())
    .with_metrics(Box::new(MyMetrics::new()))        // Observability
    .with_ttl_policy(TtlPolicy::Fixed(Duration::from_secs(3600)));  // Default TTL

// All subsequent operations use these settings
expander.with(&mut feeder, &repository, CacheStrategy::Refresh).await?;
```

**Setup-time configuration includes:**

| Method               | Purpose                           | When to Use                 |
| -------------------- | --------------------------------- | --------------------------- |
| `.with_metrics()`    | Observability and monitoring      | Production deployments      |
| `.with_ttl_policy()` | Default TTL for all cache entries | Set baseline cache duration |

**Best for:** Global policies that should apply consistently across your application.

---

### Per-Operation Configuration (Override for Specific Calls)

Per-operation configuration allows you to override settings for **individual** cache operations:

```rust
use cache_kit::OperationConfig;
use std::time::Duration;

// Create OperationConfig with custom TTL and retry
let config = OperationConfig::default()
    .with_ttl(Duration::from_secs(60))   // Override TTL for this operation only
    .with_retry(3);                      // Retry up to 3 times on failure

expander.with_config(&mut feeder, &repository, CacheStrategy::Refresh, config).await?;
```

**Per-operation configuration includes:**

| Method          | Purpose                            | When to Use                              |
| --------------- | ---------------------------------- | ---------------------------------------- |
| `.with_ttl()`   | Override TTL for this operation    | Flash sales, temporary data, A/B testing |
| `.with_retry()` | Add retry logic for this operation | Critical operations, flaky backends      |

**Best for:** Exceptional cases that need different behavior from your defaults.

---

### When to Use Each Level {#when-to-use-each-level}

#### Use Setup-Time Configuration When: {#use-setup-time-configuration-when}

✅ You want **consistent behavior** across all operations  
✅ You're setting **infrastructure concerns** (metrics, logging)  
✅ You have a **standard TTL policy** for entity types  
✅ Configuration is **environment-specific** (dev vs prod)

#### Use Per-Operation Configuration When: {#use-per-operation-configuration-when}

✅ You need **different TTL** for specific operations (e.g., flash sale prices)  
✅ You want **retry logic** for critical operations only  
✅ You're doing **A/B testing** with different cache durations  
✅ You have **special cases** that don't fit the default policy

---

### Example: Combining Both Levels

```rust
use cache_kit::{CacheExpander, OperationConfig, backend::InMemoryBackend, observability::TtlPolicy};
use std::time::Duration;

// Setup-time: Set defaults for the application
let expander = CacheExpander::new(InMemoryBackend::new())
    .with_ttl_policy(TtlPolicy::Fixed(Duration::from_secs(3600))); // 1 hour default

// Normal operation: Uses 1-hour TTL from setup
expander.with(&mut feeder, &repository, CacheStrategy::Refresh).await?;

// Special case: Override TTL for flash sale product
let flash_sale_config = OperationConfig::default()
    .with_ttl(Duration::from_secs(60));  // 1 minute for flash sale

expander
    .with_config(&mut feeder, &repository, CacheStrategy::Refresh, flash_sale_config)
    .await?;
```

**Key principle:** Setup-time configuration provides sensible defaults. Per-operation configuration handles exceptions.

---

## TTL Override Precedence

When you provide both a setup-time `ttl_policy` and a per-operation `ttl_override`, the **override takes precedence**:

| Scenario         | TTL Override | Result                           |
| ---------------- | ------------ | -------------------------------- |
| Normal operation | `None`       | Use `ttl_policy` from setup      |
| Flash sale       | `Some(60s)`  | Use `60s` (ignores setup policy) |
| Permanent data   | `Some(None)` | Could use `PerType` policy       |

### Real-World Example: E-Commerce Cache

```rust
use cache_kit::{CacheExpander, OperationConfig, observability::TtlPolicy};
use std::time::Duration;

// Setup-time: Default policy for products
let cache = CacheExpander::new(backend)
    .with_ttl_policy(TtlPolicy::PerType(|entity_type| {
        match entity_type {
            "product" => Duration::from_secs(3600),   // Normal: 1 hour
            "user" => Duration::from_secs(1800),      // User: 30 minutes
            _ => Duration::from_secs(600),            // Default: 10 minutes
        }
    }));

// Normal product: Uses 1-hour TTL from PerType policy
cache.with(&mut feeder, &repo, CacheStrategy::Refresh).await?;

// Flash sale product: Override to 5 minutes
let flash_sale_config = OperationConfig::default()
    .with_ttl(Duration::from_secs(300));  // Override beats PerType policy
cache.with_config(&mut feeder, &repo, CacheStrategy::Refresh, flash_sale_config).await?;

// Limited inventory: Override to 30 seconds
let limited_config = OperationConfig::default()
    .with_ttl(Duration::from_secs(30));   // Even shorter override
cache.with_config(&mut feeder, &repo, CacheStrategy::Refresh, limited_config).await?;
```

**How precedence works:**

```
1. If ttl_override is Some(duration) → Use it (takes precedence)
2. If ttl_override is None → Ask ttl_policy
   - PerType policy: Check entity type, use matching duration
   - Fixed policy: Use the fixed duration
   - Default policy: Let backend decide
```

---

## Putting It All Together

Here's how all concepts work together:

```rust
use cache_kit::{
    CacheEntity, CacheFeed, DataRepository, CacheService,
    backend::InMemoryBackend,
    strategy::CacheStrategy,
};
use serde::{Deserialize, Serialize};

// 1. Entity (Serializable)
#[derive(Clone, Serialize, Deserialize)]
struct Product {
    id: u64,
    name: String,
    price: f64,
}

// 2. Deterministic cache key
impl CacheEntity for Product {
    type Key = u64;
    fn cache_key(&self) -> Self::Key { self.id }
    fn cache_prefix() -> &'static str { "product" }
}

// 3. Explicit cache boundary (Feeder)
struct ProductFeeder {
    id: u64,
    product: Option<Product>,
}

impl CacheFeed<Product> for ProductFeeder {
    fn entity_id(&mut self) -> u64 { self.id }
    fn feed(&mut self, entity: Option<Product>) { self.product = entity; }
}

// 4. Data repository
struct ProductRepository;

impl DataRepository<Product> for ProductRepository {
    async fn fetch_by_id(&self, id: &u64) -> cache_kit::Result<Option<Product>> {
        // Your database logic
        Ok(Some(Product {
            id: *id,
            name: "Example Product".to_string(),
            price: 99.99,
        }))
    }
}

// Usage
#[tokio::main]
async fn main() -> cache_kit::Result<()> {
    let cache = CacheService::new(InMemoryBackend::new());
    let repository = ProductRepository;

    let mut feeder = ProductFeeder {
        id: 123,
        product: None,
    };

    // Cache operation with explicit strategy
    cache.execute(&mut feeder, &repository, CacheStrategy::Refresh).await?;

    if let Some(product) = feeder.product {
        println!("Product: {} - ${}", product.name, product.price);
    }

    Ok(())
}
```

---

## Design Philosophy

cache-kit is designed around three fundamental principles that guide every design decision:

1. **Boundaries, not ownership**
2. **Explicit behavior, not hidden magic**
3. **Integration, not lock-in**

### Boundaries, Not Ownership

cache-kit does not try to own your application stack. It integrates **around** your existing choices:

```
┌─────────────────────────────────────────┐
│           Your Choices                  │
│  • Framework (Axum, Actix, Tonic)       │
│  • ORM (SQLx, SeaORM, Diesel)           │
│  • Transport (HTTP, gRPC, Workers)      │
│  • Runtime (tokio)                      │
└──────────────┬──────────────────────────┘
               │
               ↓ Cache operations
┌─────────────────────────────────────────┐
│          cache-kit                      │
│  Places clear boundaries                │
│  Does NOT dictate architecture          │
└─────────────────────────────────────────┘
```

**What cache-kit Does vs Does NOT Do:**

| What cache-kit Does          | What cache-kit Does NOT Do    |
| ---------------------------- | ----------------------------- |
| ✅ Provide cache operations  | ❌ Replace your ORM           |
| ✅ Define cache boundaries   | ❌ Manage HTTP routing        |
| ✅ Handle serialization      | ❌ Impose web frameworks      |
| ✅ Support multiple backends | ❌ Require specific databases |
| ✅ Integrate with async      | ❌ Create runtimes            |

**Benefits:**

- **Freedom of choice** — Use any framework, ORM, transport
- **Evolutionary architecture** — Swap components independently
- **Library-safe** — Use inside SDKs and libraries
- **No vendor lock-in** — cache-kit is just one piece

### Explicit Behavior, Not Hidden Magic

cache-kit makes cache behavior **visible and predictable**. There is no implicit caching:

```rust
// ❌ WRONG: Hidden caching (magic)
fn get_user(id: &str) -> User {
    // Automatically cached somewhere?
    // How? When? For how long?
    database.query(id)
}

// ✅ RIGHT: Explicit caching (cache-kit)
fn get_user(id: &str) -> Result<Option<User>> {
    let mut feeder = UserFeeder { id: id.to_string(), user: None };

    // Explicit: I know this uses cache
    // Explicit: I chose the strategy
    // Explicit: I control the result
    cache.with(&mut feeder, &repository, CacheStrategy::Refresh)?;

    Ok(feeder.user)
}
```

**Explicit Invalidation:** cache-kit does NOT automatically invalidate on writes. You decide when to invalidate (see [Cache Ownership and Invalidation](#cache-ownership-and-invalidation) above).

**Explicit Strategies:** Four cache strategies, each with clear semantics (see [Cache Strategies](#cache-strategies) above). No guessing. No surprises.

### Integration, Not Lock-In

cache-kit is designed to **play well with others**.

**Framework Agnostic:** The same cache logic works across all frameworks:

```rust
// Axum, Actix, Tonic - all use the same cache operations
cache.with(&mut feeder, &repository, CacheStrategy::Refresh).await?;
```

**ORM Agnostic:** Works with any database layer (see [Database Compatibility](/database-compatibility) for examples).

**Backend Agnostic:** Swap backends with **zero code changes**:

```rust
// Development
let backend = InMemoryBackend::new();

// Production
let backend = RedisBackend::new(config)?;

// Same interface
let expander = CacheExpander::new(backend);
```

---

## Guarantees and Non-Guarantees

cache-kit is explicit about what it **guarantees** and what it **does not**.

### What cache-kit Guarantees

✅ **Type safety** — Compiler-verified cache operations  
✅ **Thread safety** — `Send + Sync` everywhere  
✅ **Deterministic keys** — Same entity → same key  
✅ **No silent failures** — All errors are propagated  
✅ **Backend abstraction** — Swap backends without code changes  
✅ **Async-first** — Built for tokio-based apps

### What cache-kit Does NOT Guarantee

❌ **Strong consistency** — Distributed caches are eventually consistent  
❌ **Automatic invalidation** — You control when data is invalidated  
❌ **Distributed coordination** — No locks, no consensus  
❌ **Eviction policies** — Depends on backend (Redis, Memcached)  
❌ **Persistence** — Depends on backend (Redis has persistence, Memcached doesn't)  
❌ **Cross-language compatibility** — Postcard is Rust-only

---

## Trade-Offs and Honesty

cache-kit makes intentional trade-offs and is honest about them.

### Trade-Off 1: Postcard vs JSON

| Aspect               | Postcard (Chosen) | JSON (Alternative) |
| -------------------- | ----------------- | ------------------ |
| **Performance**      | ⚡ 10-15x faster  | ❌ Baseline        |
| **Size**             | 📦 40-50% smaller | ❌ Baseline        |
| **Decimal support**  | ❌ No             | ✅ Yes             |
| **Language support** | ❌ Rust-only      | ✅ Many languages  |

**Decision:** Prioritize performance for Rust-to-Rust caching. Decimal limitation is documented and workarounds are provided. See [Serialization](/serialization) for details.

### Trade-Off 2: Async DataRepository

| Aspect                    | Async (Chosen)                        |
| ------------------------- | ------------------------------------- |
| **Native async support**  | ✅ Direct `.await`                    |
| **Modern Rust practices** | ✅ Idiomatic async/await              |
| **Compatibility**         | ✅ SQLx, SeaORM, tokio-postgres       |
| **Ecosystem alignment**   | ✅ Works with modern async frameworks |

**Decision:** Use async trait for modern async databases. This is the recommended pattern for Rust services. See [Async Programming Model](/async-model) for details.

### Trade-Off 3: Explicit Invalidation vs Automatic

| Aspect             | Explicit (Chosen) | Automatic (Alternative)        |
| ------------------ | ----------------- | ------------------------------ |
| **Control**        | ✅ Full control   | ❌ Hidden behavior             |
| **Predictability** | ✅ Predictable    | ⚠️ Can surprise you            |
| **Complexity**     | ✅ Simple         | ❌ Complex dependency tracking |

**Decision:** Make invalidation explicit. No magic, no surprises.

---

## Safety and Reliability

### Thread Safety

All cache-kit types are `Send + Sync`:

```rust
// Safe to share across threads
let cache = Arc::new(CacheExpander::new(backend));

// Safe to use in async tasks
tokio::spawn(async move {
    let mut feeder = UserFeeder { ... };
    cache.with(&mut feeder, &repo, CacheStrategy::Refresh).await?;
});
```

### Error Handling

cache-kit **never panics** in normal operation:

```rust
// All operations return Result
match cache.with(&mut feeder, &repo, CacheStrategy::Refresh).await {
    Ok(_) => println!("Success"),
    Err(e) => eprintln!("Cache error: {}", e),
}
```

### Memory Safety

- No unsafe code in cache-kit core
- All backends use safe Rust
- DashMap (InMemory) is lock-free and safe

---

## Library and SDK Use

cache-kit is **safe to use inside libraries**:

```rust
// Inside a library crate
pub struct MyLibrary {
    cache: CacheExpander<InMemoryBackend>,
    // or bring-your-own-backend pattern
}

impl MyLibrary {
    pub fn new() -> Self {
        Self {
            cache: CacheExpander::new(InMemoryBackend::new()),
        }
    }

    // Your library methods
    pub fn fetch_data(&mut self, id: &str) -> Result<Data> {
        let mut feeder = DataFeeder { ... };
        self.cache.with(&mut feeder, &self.repo, CacheStrategy::Refresh)?;
        // ...
    }
}
```

**Benefits:**

- No framework dependencies
- No global state
- No runtime assumptions
- Safe to embed

---

## When NOT to Use cache-kit

cache-kit is **not** the right choice if you need:

❌ **Distributed locks** — Use a coordination service (etcd, ZooKeeper)  
❌ **Strong consistency** — Use a distributed database (Spanner, CockroachDB)  
❌ **Cross-language caching** — Use JSON or MessagePack (when available)  
❌ **Automatic schema migration** — cache-kit uses explicit versioning  
❌ **All-in-one framework** — cache-kit is just a caching library

---

## Next Steps

- [Installation](/installation) — Get started with cache-kit
- [Database Compatibility](/database-compatibility) — Integration examples
- [Async Programming Model](/async-model) — Understanding async-first design
- [API Frameworks](/api-frameworks) — Using with Axum, Actix, gRPC
- [Serialization](/serialization) — Postcard and serialization options
- [Cache Backends](/backends) — Redis, Memcached, InMemory
- Explore the [Actix + SQLx reference implementation](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)
