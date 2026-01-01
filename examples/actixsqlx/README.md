# actixsqlx: REST API with cache-kit

A realistic REST API example using **Actix Web**, **SQLx**, **PostgreSQL**, and **cache-kit**.

This example demonstrates:

- ✅ Multi-table relationships (User and Product entities)
- ✅ Async database access with SQLx and migrations
- ✅ REST API endpoints with Actix Web
- ✅ Cache-kit integration using Service Layer pattern
- ✅ Postcard serialization
- ✅ Clean separation of concerns (Routes → Services → Repositories)

## Prerequisites

- Rust 1.75+
- PostgreSQL 18+
- Docker (for running Postgres)

## Setup

### 1. Start PostgreSQL

Using Docker:

```bash
make up
```

### 2. Build the Example

See available targets with `make help`. To build:

```bash
cargo build
```

## Running the Server

```bash
cargo run
```

You should see:

```
Starting server at http://127.0.0.1:8080
```

The server runs on port `8080`.

## API Endpoints

| Method | Endpoint        | Description    | Cache Strategy               |
| ------ | --------------- | -------------- | ---------------------------- |
| GET    | `/health`       | Health check   | None                         |
| GET    | `/users/:id`    | Get user       | Refresh (cache + DB)         |
| POST   | `/users`        | Create user    | Refresh (cache after create) |
| PUT    | `/users/:id`    | Update user    | Invalidate + Refresh         |
| DELETE | `/users/:id`    | Delete user    | Invalidate                   |
| GET    | `/products/:id` | Get product    | Refresh (cache + DB)         |
| POST   | `/products`     | Create product | Refresh (cache after create) |
| PUT    | `/products/:id` | Update product | Invalidate + Refresh         |
| DELETE | `/products/:id` | Delete product | Invalidate                   |

### Testing with curl

```bash
# Health check
curl http://localhost:8080/health

# Get user
curl http://localhost:8080/users/user_001

# Create user
curl -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"id":"user_003","username":"charlie","email":"charlie@example.com","created_at":"2025-12-26T00:00:00Z"}'
```

## [Database Schema](./migrations/)

## Key Design Decisions

### Service Layer Pattern

This example uses the **Service Layer** architecture pattern:

- **Routes**: Handle HTTP requests/responses only, delegate to services
- **Services**: Contain business logic and cache integration
- **Repositories**: Pure database access, no cache awareness

This provides clear separation of concerns and makes the code testable and maintainable.

### Cache Strategies

The example demonstrates:

1. **Refresh** (GET operations)

   - Check cache first
   - On miss, fetch from DB and store in cache
   - On hit, return from cache

2. **Invalidate** (PUT/DELETE operations)
   - Remove from cache
   - Force fresh fetch on next read

## File Structure

```
src/
├── main.rs              # Server setup
├── models.rs            # User, Product entities
├── repository.rs        # Database access (no cache)
├── routes.rs            # HTTP handlers
├── error.rs             # Error types
├── lib.rs               # Library exports
└── services/
    ├── mod.rs
    ├── user_service.rs      # User business logic + cache
    └── product_service.rs   # Product business logic + cache

migrations/
├── 001_create_users.sql
└── 002_create_products.sql
```

## Running Tests

Run the complete test suite (includes database setup, migrations, and seeded data):

```bash
make test
```

This validates:

- Database migrations
- Cache hit/miss behavior
- CRUD cycles with cache invalidation

## License

MIT
