use cache_kit::observability::CacheMetrics;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Simple metrics collection for cache operations
/// Implements the cache-kit library's CacheMetrics trait
#[derive(Clone)]
pub struct PrometheusMetrics {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    latency_total_us: Arc<AtomicU64>,
    latency_count: Arc<AtomicU64>,
}

impl CacheMetrics for PrometheusMetrics {
    /// Record a cache hit (called automatically by cache-kit)
    fn record_hit(&self, _key: &str, duration: Duration) {
        let latency_us = duration.as_micros() as u64;
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.latency_total_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss (called automatically by cache-kit)
    fn record_miss(&self, _key: &str, duration: Duration) {
        let latency_us = duration.as_micros() as u64;
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.latency_total_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache error (called automatically by cache-kit)
    fn record_error(&self, _key: &str, _error: &str) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache set operation (optional, called automatically by cache-kit)
    fn record_set(&self, _key: &str, duration: Duration) {
        let latency_us = duration.as_micros() as u64;
        self.latency_total_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache delete operation (optional, called automatically by cache-kit)
    fn record_delete(&self, _key: &str, duration: Duration) {
        let latency_us = duration.as_micros() as u64;
        self.latency_total_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl PrometheusMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        PrometheusMetrics {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            latency_total_us: Arc::new(AtomicU64::new(0)),
            latency_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get cache hit rate (0.0 to 1.0)
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

    /// Render metrics in Prometheus format
    pub fn render_prometheus(&self) -> String {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);

        format!(
            r#"# HELP cache_hits_total Total cache hits
# TYPE cache_hits_total counter
cache_hits_total {}

# HELP cache_misses_total Total cache misses
# TYPE cache_misses_total counter
cache_misses_total {}

# HELP cache_errors_total Total cache errors
# TYPE cache_errors_total counter
cache_errors_total {}

# HELP cache_hit_rate Cache hit rate (0.0-1.0)
# TYPE cache_hit_rate gauge
cache_hit_rate {:.4}

# HELP cache_error_rate Cache error rate (0.0-1.0)
# TYPE cache_error_rate gauge
cache_error_rate {:.4}

# HELP cache_avg_latency_us Average cache latency in microseconds
# TYPE cache_avg_latency_us gauge
cache_avg_latency_us {:.2}

# HELP cache_total_ops Total cache operations
# TYPE cache_total_ops counter
cache_total_ops {}
"#,
            hits,
            misses,
            errors,
            self.hit_rate(),
            self.error_rate(),
            self.avg_latency_us(),
            self.total_ops()
        )
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}
