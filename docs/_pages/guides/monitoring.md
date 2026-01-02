---
layout: single
title: Monitoring & Metrics
description: "Set up production-grade monitoring, metrics, and observability for cache-kit"
parent: Guides
nav_order: 12
date: 2025-12-31
---

Monitoring cache-kit allows you to:

- **Detect problems early** — Catch issues before users notice
- **Understand performance** — Hit rates, latency, throughput
- **Optimize configuration** — Data-driven tuning decisions
- **Alert on degradation** — Automated on-call notifications
- **Troubleshoot quickly** — Historical data for diagnosis

### The 4 Golden Signals for Caching

1. **Latency** — How fast are cache operations? (p50, p99)
2. **Traffic** — How much are we using the cache? (ops/sec)
3. **Errors** — What percentage of operations fail? (error rate)
4. **Hit rate** — What percentage of requests hit cache? (cache efficiency)

---

## Metrics Implementation

### Simple Metrics Struct

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct CacheMetrics {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    latency_total_us: Arc<AtomicU64>,
    latency_count: Arc<AtomicU64>,
}

impl CacheMetrics {
    pub fn new() -> Self {
        CacheMetrics {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            latency_total_us: Arc::new(AtomicU64::new(0)),
            latency_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a cache hit
    pub fn record_hit(&self, latency_us: u64) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.latency_total_us.fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub fn record_miss(&self, latency_us: u64) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.latency_total_us.fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache error
    pub fn record_error(&self, latency_us: u64) {
        self.errors.fetch_add(1, Ordering::Relaxed);
        self.latency_total_us.fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        if (hits + misses) == 0.0 {
            return 0.0;
        }
        hits / (hits + misses)
    }

    /// Get error rate (0.0 to 1.0)
    pub fn error_rate(&self) -> f64 {
        let errors = self.errors.load(Ordering::Relaxed) as f64;
        let total = self.hits.load(Ordering::Relaxed) as f64
                  + self.misses.load(Ordering::Relaxed) as f64
                  + errors;
        if total == 0.0 {
            return 0.0;
        }
        errors / total
    }

    /// Get average latency in microseconds
    pub fn avg_latency_us(&self) -> f64 {
        let count = self.latency_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total = self.latency_total_us.load(Ordering::Relaxed) as f64;
        total / count as f64
    }

    /// Get total operations
    pub fn total_ops(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
            + self.misses.load(Ordering::Relaxed)
            + self.errors.load(Ordering::Relaxed)
    }

    /// Get throughput (ops/sec) — pass elapsed_secs
    pub fn throughput_ops_sec(&self, elapsed_secs: f64) -> f64 {
        self.total_ops() as f64 / elapsed_secs
    }
}
```

### Instrument Your Code

```rust
pub async fn get_user_with_metrics(
    cache: &mut CacheExpander<impl CacheBackend>,
    repo: &UserRepository,
    user_id: String,
    metrics: &CacheMetrics,
) -> Result<Option<User>> {
    let start = Instant::now();
    let mut feeder = UserFeeder {
        id: user_id.clone(),
        user: None,
    };

    match cache.with(&mut feeder, repo, CacheStrategy::Refresh) {
        Ok(_) => {
            let latency_us = start.elapsed().as_micros() as u64;
            if feeder.user.is_some() {
                metrics.record_hit(latency_us);
                info!("Cache HIT for user {}", user_id);
            } else {
                metrics.record_miss(latency_us);
                info!("Cache MISS for user {}", user_id);
            }
            Ok(feeder.user)
        }
        Err(e) => {
            let latency_us = start.elapsed().as_micros() as u64;
            metrics.record_error(latency_us);
            error!("Cache ERROR for user {}: {}", user_id, e);
            Err(e)
        }
    }
}
```

---

## Prometheus Integration

### Expose Metrics Endpoint

See the [Axum example](https://github.com/megamsys/cache-kit.rs/tree/main/examples/axummetrics) for a complete working implementation with:

- Metrics HTTP endpoint
- API server with cache instrumentation
- Prometheus scrape configuration

The metrics endpoint exposes the standard Prometheus format:

```
cache_hits_total (counter)
cache_misses_total (counter)
cache_errors_total (counter)
cache_hit_rate (gauge, 0.0-1.0)
cache_error_rate (gauge, 0.0-1.0)
cache_avg_latency_us (gauge)
```

### Prometheus Scrape Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: "cache-kit"
    static_configs:
      - targets: ["localhost:3000"]
    metrics_path: "/metrics"
```

### Alert Rules

{% raw %}

```yaml
# alerts.yml
groups:
  - name: cache-kit
    interval: 30s
    rules:
      # Alert if hit rate drops below 30%
      - alert: LowCacheHitRate
        expr: cache_hit_rate < 0.3
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Low cache hit rate ({{ $value | humanizePercentage }})"
          description: "Cache hit rate below 30% for 5 minutes"

      # Alert if error rate exceeds 5%
      - alert: HighCacheErrorRate
        expr: cache_error_rate > 0.05
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High cache error rate ({{ $value | humanizePercentage }})"
          description: "Cache errors above 5% - likely backend down"

      # Alert if average latency exceeds 100ms
      - alert: SlowCacheLatency
        expr: cache_avg_latency_us > 0.9.00
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Slow cache latency ({{ $value | humanizeDuration }})"
          description: "Cache operations averaging > 100ms"
```

{% endraw %}

---

## Key Metrics to Monitor

### Business Metrics

| Metric         | Target   | Alert >    |
| -------------- | -------- | ---------- |
| Cache Hit Rate | > 70%    | < 30%      |
| Error Rate     | < 1%     | > 5%       |
| P99 Latency    | < 50ms   | > 100ms    |
| Throughput     | Baseline | Drop > 20% |

### System Metrics

| Metric                 | Target  | Alert >            |
| ---------------------- | ------- | ------------------ |
| Connection Pool Active | < max   | == max (5 min)     |
| Memory Usage           | Stable  | +50% from baseline |
| Cache Size             | Bounded | Growing unbounded  |
| Evictions              | None    | > 0/min            |

---

## Next Steps

- Check [Troubleshooting guide](/guides/troubleshooting) for diagnosis patterns
- Review [Backends guide](/backends) for backend-specific metrics
