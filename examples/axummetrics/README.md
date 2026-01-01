# Axum + cache-kit Monitoring Example

Complete example demonstrating integrating cache-kit with Axum web framework and exposing Prometheus metrics.

## What's Included

- **main.rs** — REST API server with cache-kit integration

  - GET `/api/user/:id` — Fetch user with automatic caching
  - GET `/health` — Health check endpoint
  - GET `/metrics` — Prometheus metrics endpoint

- **metrics.rs** — Metrics collection module
  - Atomic counters for hits, misses, errors
  - Latency tracking (average, per-operation)
  - Prometheus format output
  - Hit rate, error rate calculations

## Running the Example

```bash
make run
```

Server starts on `http://127.0.0.1:3000`.

## Testing the API

### Fetch a user (will be cached)

```bash
curl http://127.0.0.1:3000/api/user/user_001
```

### Health check

```bash
curl http://127.0.0.1:3000/health
```

### View metrics

```bash
curl http://127.0.0.1:3000/metrics
```

The `/metrics` endpoint returns Prometheus-compatible metrics:

- `cache_hits_total` — Total cache hits
- `cache_misses_total` — Total cache misses
- `cache_errors_total` — Total cache errors
- `cache_hit_rate` — Current hit rate (0.0-1.0)
- `cache_error_rate` — Current error rate (0.0-1.0)
- `cache_avg_latency_us` — Average operation latency in microseconds

## Available Users

The mock repository has these test users:

- `user_001` — Alice Johnson
- `user_002` — Bob Smith
- `user_003` — Charlie Brown

## Integration with Prometheus

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: "cache-kit-example"
    static_configs:
      - targets: ["127.0.0.1:3000"]
    metrics_path: "/metrics"
    scrape_interval: 15s
```

## Implementation Notes

- Uses **InMemoryBackend** for simplicity (can be swapped for Redis/Memcached)
- Metrics are collected with atomic operations (no locks on critical path)
- Each API request records hit/miss/error with latency
- Demonstrates best practice: separate metrics collection from business logic

## Related Documentation

See [Monitoring Guide](https://cachekit.org/guides/monitoring.md) for detailed setup instructions and Prometheus configuration.
