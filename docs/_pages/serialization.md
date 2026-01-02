---
layout: single
title: Serialization Support
description: "Understanding serialization formats and limitations in cache-kit"
permalink: /serialization/
nav_order: 7
date: 2025-12-26
---

---

## ⚠️ Critical Limitation

**Decimal types (`rust_decimal::Decimal`, `bigdecimal::BigDecimal`) are NOT supported by Postcard serialization.**

If your entities use Decimal fields (common in financial apps), you MUST convert to `String` or `i64` before caching. See [Decimal Types Not Supported](#decimal-types-not-supported) below.

---

## Serialization as a First-Class Concern

cache-kit treats serialization as a **first-class architectural concern**.

Serialization determines:

- **Storage format** in the cache backend
- **Performance** characteristics (speed, size)
- **Type support** (which Rust types can be cached)
- **Interoperability** (can other languages read the cache?)

---

## Supported Formats

### Tier-1: Postcard (Recommended)

**Postcard** is the primary recommended serialization format for cache-kit.

| Feature              | Postcard                               |
| -------------------- | -------------------------------------- |
| **Performance**      | ⚡ Very fast (10-15x faster than JSON) |
| **Size**             | 📦 Compact (40-50% smaller than JSON)  |
| **Type safety**      | ✅ Strong Rust type preservation       |
| **Determinism**      | ✅ Same input → same output            |
| **Language support** | ❌ Rust-only                           |
| **Decimal support**  | ❌ No (see limitations below)          |

#### Why Postcard?

- **Optimized for Rust** — Zero-copy deserialization where possible
- **Explicit versioning** — Simple versioning with automatic cache invalidation
- **Minimal overhead** — Field order matters, no field names stored
- **Fast** — Designed for embedded and performance-critical systems

#### Installation

Postcard is included by default:

```toml
[dependencies]
cache-kit = { version = "0.9" }
```

#### Usage

No explicit configuration needed — cache-kit uses Postcard automatically:

```rust
use cache_kit::CacheEntity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    age: u32,
}

impl CacheEntity for User {
    type Key = String;
    fn cache_key(&self) -> Self::Key { self.id.clone() }
    fn cache_prefix() -> &'static str { "user" }
}

// Serialization to Postcard happens automatically
```

---

### Tier-2: MessagePack (Planned)

**MessagePack** will be available as an alternative serialization format.

| Feature              | MessagePack (Planned)              |
| -------------------- | ---------------------------------- |
| **Performance**      | ⚡ Fast (4-6x faster than JSON)    |
| **Size**             | 📦 Compact (50% smaller than JSON) |
| **Type safety**      | ⚠️ Partial                         |
| **Determinism**      | ⚠️ Partial (field order varies)    |
| **Language support** | ✅ Many languages                  |
| **Decimal support**  | ⚠️ Depends on implementation       |

**Community contributions welcome!** Help us add MessagePack support.

---

## Serialization Characteristics

### Postcard: Binary, Deterministic

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Product {
    id: u64,          // 8 bytes (compact)
    name: String,     // length-prefixed
    price: f64,       // 8 bytes (IEEE 754)
}

// Serialized format (example):
// [id: 8 bytes][name_len: varint][name: UTF-8 bytes][price: 8 bytes]
```

**Key property:** Serializing the same value twice produces **identical bytes**.

```rust
let product1 = Product { id: 123, name: "Widget".to_string(), price: 99.99 };
let product2 = Product { id: 123, name: "Widget".to_string(), price: 99.99 };

let bytes1 = postcard::to_allocvec(&product1)?;
let bytes2 = postcard::to_allocvec(&product2)?;

assert_eq!(bytes1, bytes2);  // ✅ Always true
```

This enables:

- **Reliable cache keys** based on content
- **Deduplication** in distributed caches
- **Reproducible testing**

---

## Known Limitations

### Decimal Types Not Supported

Postcard (and many binary formats) do **not support** arbitrary-precision decimal types out of the box.

Affected types:

- `rust_decimal::Decimal`
- `bigdecimal::BigDecimal`
- Database `NUMERIC` / `DECIMAL` columns

#### Why This Limitation Exists

Binary formats like Postcard serialize types based on their in-memory representation. Decimal types have complex internal structures that don't map cleanly to portable binary formats.

#### Workaround Strategies

##### Strategy 1: Convert to Supported Primitives

Store monetary values as **integer cents** instead of decimal dollars:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct Product {
    id: String,
    name: String,
    price_cents: i64,  // ✅ Store $99.99 as 9999 cents
}

impl Product {
    pub fn price_dollars(&self) -> f64 {
        self.price_cents as f64 / 100.0
    }

    pub fn set_price_dollars(&mut self, dollars: f64) {
        self.price_cents = (dollars * 100.0).round() as i64;
    }
}
```

**Pros:**

- ✅ No precision loss for monetary values
- ✅ Fast serialization
- ✅ Compact storage

**Cons:**

- ❌ Manual conversion needed
- ❌ Limited to representable range of `i64`

##### Strategy 2: String Representation

Store decimals as strings (not recommended for performance):

```rust
#[derive(Clone, Serialize, Deserialize)]
struct Product {
    id: String,
    name: String,
    price: String,  // "99.99" as string
}
```

**Pros:**

- ✅ No precision loss
- ✅ Preserves exact decimal representation

**Cons:**

- ❌ Slower serialization
- ❌ Larger storage footprint
- ❌ Manual parsing required

---

## Serialization Best Practices

### DO

- ✅ Use primitive types where possible (`i64`, `f64`, `String`)
- ✅ Convert decimals to integers (cents) for monetary values
- ✅ Create cache-specific DTOs if needed
- ✅ Document conversion logic clearly
- ✅ Test roundtrip serialization

### DON'T

- ❌ Assume all Rust types are serializable
- ❌ Mix database types with cache types without conversion
- ❌ Ignore serialization errors
- ❌ Use `unwrap()` on deserialization
- ❌ Store sensitive data without encryption

---

## Versioning and Schema Evolution

cache-kit uses **explicit versioning** for cached data.

### Current Approach

cache-kit wraps all cached entries in a versioned envelope:

```
[MAGIC (4 bytes)] [VERSION (4 bytes)] [POSTCARD PAYLOAD]
```

- **MAGIC:** `b"CKIT"` — Identifies cache-kit entries
- **VERSION:** `u32` — Schema version number
- **PAYLOAD:** Postcard-serialized entity

### Version Mismatches

When the schema version changes:

1. **Old entries rejected** — Cannot be deserialized
2. **Cache miss triggered** — Fetch from database
3. **New entry cached** — With updated version

**No migration** — Cache naturally repopulates with new schema.

### Handling Schema Changes

When you modify an entity structure:

```rust
// Version 1
struct User {
    id: String,
    name: String,
}

// Version 2 (added field)
struct User {
    id: String,
    name: String,
    email: Option<String>,  // New field
}
```

**What happens:**

1. Deploy your code with the new entity structure
2. Old cache entries will fail to deserialize (treated as cache misses)
3. Cache will automatically refetch from database and store with new structure
4. No manual intervention needed — cache naturally repopulates

**Note:** The schema version is managed internally by cache-kit. When deserialization fails due to structure changes, entries are automatically treated as cache misses and refetched.

---

## Troubleshooting

### Error: "Serialization failed"

**Cause:** Entity contains unsupported types (e.g., `Decimal`)

**Solution:** Convert to supported primitives

### Error: "Version mismatch"

**Cause:** Cached entry has different schema version

**Solution:** This is expected after schema changes. Entry will be invalidated and refetched.

### Error: "Invalid magic header"

**Cause:** Cache entry is corrupted or not created by cache-kit

**Solution:** Clear the cache key manually or let it expire

---

## Next Steps

- Learn about [Cache backend options](/cache-kit.rs/backends)
- Review [Core Concepts](/cache-kit.rs/concepts) — Design philosophy and principles
- Explore the [Actix + SQLx reference implementation](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)
- **Contribute MessagePack support!** See [CONTRIBUTING.md](https://github.com/megamsys/cache-kit.rs/blob/main/CONTRIBUTING.md)
