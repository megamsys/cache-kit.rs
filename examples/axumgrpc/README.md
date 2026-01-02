# axumgrpc: Invoice API with cache-kit

A realistic gRPC-only example using **Tonic**, **SQLx**, **PostgreSQL**, and **cache-kit**.

This example demonstrates:

- ✅ Multi-table relationships (Customer → Invoice → LineItems)
- ✅ Async database access with SQLx and migrations
- ✅ gRPC service definitions and handlers
- ✅ Cache-kit integration for invoice fetching
- ✅ Postcard serialization with monetary types (using `i64` cents instead of `Decimal`)
- ✅ gRPC-only service (no REST endpoints)

## Prerequisites

- Rust 1.75+
- PostgreSQL 18+
- Docker (for running Postgres)
- **protoc** (required for gRPC proto compilation)
  - macOS: `brew install protobuf`
  - Linux: `apt-get install protobuf-compiler` (Ubuntu/Debian) or equivalent
  - Or download from: https://github.com/protocolbuffers/protobuf/releases

## Setup

### 1. Start PostgreSQL

Using Docker:

```bash
make up
```

### 3. Install Dependencies

```bash
make build
```

## Running the Server

```bash
make run
```

You should see:

```
Migrations completed
Starting gRPC-only server...
gRPC server listening on 127.0.0.1:50051
```

## gRPC Server

The server runs on port `50051` as a gRPC-only service.

### Testing with grpcurl (Most Popular CLI Tool)

`grpcurl` is the most popular command-line tool for testing gRPC endpoints, similar to `curl` for REST APIs.

**Installation:**

```bash
# macOS:
brew install grpcurl

# Linux (with Go installed):
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# Or download precompiled binaries from:
# https://github.com/fullstorydev/grpcurl/releases
```

**Usage:**

Since this server doesn't enable gRPC reflection, you need to specify the proto file:

```bash
# Get a specific invoice (uses seeded test data)
grpcurl -plaintext \
  -proto proto/invoices.proto \
  -d '{"invoice_id": "019b747b-a331-73b3-acfe-867a5d0c3ded"}' \
  127.0.0.1:50051 invoices.InvoicesService/GetInvoice

# List invoices for a customer (uses seeded test data)
grpcurl -plaintext \
  -proto proto/invoices.proto \
  -d '{"customer_id": "550e8400-e29b-41d4-a716-446655440000", "limit": 10, "offset": 0}' \
  127.0.0.1:50051 invoices.InvoicesService/ListInvoices

# List all available services and methods
grpcurl -plaintext \
  -proto proto/invoices.proto \
  127.0.0.1:50051 list

# Describe a service
grpcurl -plaintext \
  -proto proto/invoices.proto \
  127.0.0.1:50051 describe invoices.InvoicesService
```

### Health Checks

This example is a gRPC-only service. For health checks, gRPC has a built-in [Health Checking Protocol](https://github.com/grpc/grpc/blob/master/doc/health-checking.md) that you can implement using `tonic_health`. This is the standard way to do health checks in gRPC services and works well with Kubernetes and load balancers.

## [Database Schema](./migrations/)

## Key Design Decisions

### Decimal vs i64 for Money

**Problem:** Postcard does not support `rust_decimal::Decimal` natively. This example uses `i64` for cents instead.

**Solution:** Store all monetary values as `i64` (cents):

```rust
// Instead of: Decimal::from_str("99.99")
// Use: 9999i64 (representing $99.99)
```

### Async Database Access

Cache-kit supports async operations. This example uses `async fn fetch_by_id` to query the database:

```rust
impl DataRepository<Invoice> for InvoiceRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<Invoice>> {
        let invoice_id = uuid::Uuid::parse_str(id)
            .map_err(|e| cache_kit::Error::BackendError(format!("Invalid UUID: {}", e)))?;
        Database::get_invoice(&self.pool, &invoice_id)
            .await
            .map_err(|e| cache_kit::Error::BackendError(e.to_string()))
    }
}
```

### Cache Strategy

The example integrates cache-kit for:

- Single invoice caching by ID (used in `GetInvoice`)
- Cache invalidation on status updates (used in `UpdateInvoiceStatus`)

**Note:** The `ListInvoices` method currently does not use caching - it queries the database directly. A helper function `invoice_list_cache_key` exists in `cache_config.rs` for future implementation.

See `src/cache_config.rs` for cache key generation.

## File Structure

```
src/
├── main.rs           # Server setup (gRPC-only)
├── db.rs             # Database queries via SQLx
├── models.rs         # Invoice, Customer, LineItem types
├── cache_config.rs   # CacheEntity implementations
├── repository.rs     # DataRepository for cache-kit
└── grpc.rs           # gRPC service handler

proto/
└── invoices.proto    # gRPC service definitions

migrations/
├── 001_init_schema.sql
├── 002_add_indexes.sql
└── 003_seed_data.sql
```

## Running Tests

Run the complete test suite (includes database setup, migrations, and seeded data):

```bash
make test
```

This validates:

- Database migrations and seeded data loading
- Cache hit/miss behavior
- CRUD cycles with cache invalidation

## License

MIT
