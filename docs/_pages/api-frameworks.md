---
layout: single
title: API Frameworks & Transport Layers
description: "Using cache-kit across different API frameworks and transport protocols"
permalink: /api-frameworks/
nav_order: 9
date: 2025-12-28
---

---

cache-kit is framework-agnostic and works with any framework or transport. For the design philosophy behind this approach, see [Core Concepts](/concepts#integration-not-lock-in).

---

## Framework Layer vs Transport Layer

cache-kit distinguishes between **framework** (application structure) and **transport** (communication protocol).

### Framework Layer

Frameworks provide application structure:

- Request routing
- Middleware
- State management
- Error handling

### Transport Layer

Transports handle communication:

- HTTP (REST)
- gRPC (Protocol Buffers)
- WebSockets
- Message queues

**cache-kit sits below both layers**, operating on domain entities regardless of how they're exposed.

---

## Conceptual Separation

```
┌─────────────────────────────────────────┐
│         Transport Layer                 │
│  (HTTP / gRPC / WebSocket / Workers)    │
└──────────────┬──────────────────────────┘
               │ Request/Response DTOs
               ↓
┌─────────────────────────────────────────┐
│        Framework Layer                  │
│     (Axum / Actix / Tonic / Tower)      │
└──────────────┬──────────────────────────┘
               │ Extract params
               ↓
┌─────────────────────────────────────────┐
│         Service Layer                   │
│     (Business logic + cache-kit)        │
└──────────────┬──────────────────────────┘
               │ Domain entities
               ↓
┌─────────────────────────────────────────┐
│      Repository Layer                   │
│        (Database / ORM)                 │
└─────────────────────────────────────────┘
```

**Key principle:** Transport must never leak into cache or business logic. Cached logic should be reusable across transports.

---

## Axum Integration (Recommended)

Axum is a modern, ergonomic web framework built on tokio and tower.

### Installation

```toml
[dependencies]
cache-kit = { version = "0.9" }
axum = {version = "0.8" }
tokio = { version = "1.41", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### Complete Example

- **[examples/axummetrics](https://github.com/megamsys/cache-kit.rs/tree/main/examples/axummetrics)** — Complete Axum integration with cache-kit including REST API handlers, state management, and Prometheus metrics

---

## Actix Web Integration

Actix is a mature, high-performance web framework.

**Complete example:**

- **[examples/actixsqlx](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx)** — Full Actix Web integration with cache-kit including:
  - Service layer pattern
  - PostgreSQL + SQLx integration
  - CRUD operations with caching
  - Docker Compose setup
  - Production-ready error handling
  - REST API handlers

---

## gRPC with Tonic

gRPC services can use cache-kit for caching database entities before serializing to Protocol Buffers.

### Installation

```toml
[dependencies]
cache-kit = { version ="0.9" }
tonic = { version = "0.14" }
prost = { version = "0.14" }
tokio = { version = "1.41", features = ["full"] }
```

### gRPC Service Implementation

- **[examples/axumgrpc](https://github.com/megamsys/cache-kit.rs/tree/main/examples/axumgrpc)** — Complete gRPC integration with cache-kit using Tonic, including:
  - Service handlers
  - Protocol Buffer definitions
  - SQLx database integration
  - Cache invalidation patterns

---

## Reusable Service Layer

Define business logic once, use across transports. This pattern keeps cache logic in the service layer, making it reusable across HTTP, gRPC, and other transports.

**Example implementation:**

- **[examples/actixsqlx/src/services/user_service.rs](https://github.com/megamsys/cache-kit.rs/tree/main/examples/actixsqlx/src/services/user_service.rs)** — Complete service layer with cache-kit integration, including CRUD operations with caching and cache invalidation

---

## Best Practices

### DO

- ✅ Keep cache logic in service layer
- ✅ Reuse services across transports
- ✅ Separate DTOs from domain entities
- ✅ Handle cache errors gracefully at API boundary

### DON'T

- ❌ Put cache calls directly in HTTP handlers
- ❌ Leak HTTP concepts into service layer
- ❌ Cache transport-specific data (headers, status codes)
- ❌ Mix serialization formats (use domain entities, not transport DTOs)

---

## Next Steps

- Learn about [Core Concepts](/concepts) — Understanding cache-kit's design philosophy
- Explore [Database & ORM Compatibility](/database-compatibility) — ORM integration examples
- Review [Async Programming Model](/async-model) — Async-first design
- See [Serialization formats](/serialization) — Postcard and serialization options
- Explore [Cache backend options](/backends) — Redis, Memcached, InMemory
