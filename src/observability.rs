//! Observability and metrics collection for cache operations.
//!
//! This module provides traits and implementations for monitoring cache behavior,
//! tracking performance metrics, and managing cache entry time-to-live (TTL) policies.
//!
//! # Module Overview
//!
//! Cache-kit separates observability into two concerns:
//!
//! - **Metrics (`CacheMetrics`)**: Track hits, misses, performance timing
//! - **TTL Policies (`TtlPolicy`)**: Control how long entries remain in cache
//!
//! # Metrics
//!
//! Implement the `CacheMetrics` trait to collect cache statistics for your monitoring system:
//!
//! ```ignore
//! use cache_kit::observability::CacheMetrics;
//! use std::time::Duration;
//!
//! struct PrometheusMetrics;
//!
//! impl CacheMetrics for PrometheusMetrics {
//!     fn record_hit(&self, _key: &str, _duration: Duration) {
//!         // Update your metrics backend
//!         // counter!("cache_hits").inc();
//!         // histogram!("cache_latency").record(duration);
//!     }
//!     // ... implement other methods
//! }
//!
//! // let expander = CacheExpander::new(backend)
//! //     .with_metrics(Box::new(PrometheusMetrics));
//! ```
//!
//! Default behavior (if not overridden) uses `NoOpMetrics`, which logs via the `log` crate.
//!
//! # TTL Policies
//!
//! Control cache entry lifespan with flexible TTL policies:
//!
//! ```
//! use cache_kit::observability::TtlPolicy;
//! use std::time::Duration;
//!
//! // Fixed TTL for all entries (5 minutes)
//! let _policy = TtlPolicy::Fixed(Duration::from_secs(300));
//!
//! // Different TTL per entity type
//! let _policy = TtlPolicy::PerType(|entity_type| {
//!     match entity_type {
//!         "user" => Duration::from_secs(3600),      // 1 hour
//!         "session" => Duration::from_secs(1800),   // 30 minutes
//!         _ => Duration::from_secs(600),            // 10 minutes default
//!     }
//! });
//!
//! // let expander = CacheExpander::new(backend)
//! //     .with_ttl_policy(_policy);
//! ```
//!
//! # When to Use Each TTL Policy
//!
//! | Policy | Use Case | Example |
//! |--------|----------|---------|
//! | `Default` | Let backend decide | Works with Redis default TTL |
//! | `Fixed` | Uniform cache duration | All entries expire in 5 minutes |
//! | `Infinite` | Never expire | Static reference data (rarely used) |
//! | `PerType` | Type-specific expiry | Users cache 1h, sessions 30m |
//!
//! # Metrics Methods
//!
//! The `CacheMetrics` trait provides hooks for all cache lifecycle events:
//! - `record_hit()` - Cache hit with operation duration
//! - `record_miss()` - Cache miss with operation duration
//! - `record_set()` - Cache write with operation duration
//! - `record_delete()` - Cache delete with operation duration
//! - `record_error()` - Operation failure with error message
//!
//! All methods receive the cache key and relevant timing/error information.

use std::time::Duration;

/// Trait for cache metrics collection.
pub trait CacheMetrics: Send + Sync {
    /// Record a cache hit.
    fn record_hit(&self, key: &str, duration: Duration) {
        debug!("Cache HIT: {} took {:?}", key, duration);
    }

    /// Record a cache miss.
    fn record_miss(&self, key: &str, duration: Duration) {
        debug!("Cache MISS: {} took {:?}", key, duration);
    }

    /// Record a cache set operation.
    fn record_set(&self, key: &str, duration: Duration) {
        debug!("Cache SET: {} took {:?}", key, duration);
    }

    /// Record a cache delete operation.
    fn record_delete(&self, key: &str, duration: Duration) {
        debug!("Cache DELETE: {} took {:?}", key, duration);
    }

    /// Record an error.
    fn record_error(&self, key: &str, error: &str) {
        warn!("Cache ERROR for {}: {}", key, error);
    }
}

/// Default metrics implementation (no-op).
#[derive(Clone, Default)]
pub struct NoOpMetrics;

impl CacheMetrics for NoOpMetrics {
    fn record_hit(&self, _key: &str, _duration: Duration) {}
    fn record_miss(&self, _key: &str, _duration: Duration) {}
    fn record_set(&self, _key: &str, _duration: Duration) {}
    fn record_delete(&self, _key: &str, _duration: Duration) {}
    fn record_error(&self, _key: &str, _error: &str) {}
}

/// TTL (Time-to-Live) policy for cache entries.
#[derive(Clone, Debug, Default)]
pub enum TtlPolicy {
    /// Use backend's default TTL
    #[default]
    Default,

    /// Fixed duration for all entries
    Fixed(Duration),

    /// No TTL (entries live forever)
    Infinite,

    /// Custom per-type policy
    PerType(fn(&str) -> Duration),
}

impl TtlPolicy {
    /// Get TTL for an entity type.
    pub fn get_ttl(&self, entity_type: &str) -> Option<Duration> {
        match self {
            TtlPolicy::Default => None,
            TtlPolicy::Fixed(d) => Some(*d),
            TtlPolicy::Infinite => None,
            TtlPolicy::PerType(f) => Some(f(entity_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_metrics() {
        let metrics = NoOpMetrics;
        metrics.record_hit("key", Duration::from_secs(1));
        metrics.record_miss("key", Duration::from_secs(2));
    }

    #[test]
    fn test_ttl_policy_default() {
        let policy = TtlPolicy::Default;
        assert_eq!(policy.get_ttl("any"), None);
    }

    #[test]
    fn test_ttl_policy_fixed() {
        let policy = TtlPolicy::Fixed(Duration::from_secs(300));
        assert_eq!(policy.get_ttl("any"), Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_ttl_policy_per_type() {
        let policy = TtlPolicy::PerType(|entity_type| match entity_type {
            "employment" => Duration::from_secs(3600),
            _ => Duration::from_secs(1800),
        });

        assert_eq!(
            policy.get_ttl("employment"),
            Some(Duration::from_secs(3600))
        );
        assert_eq!(policy.get_ttl("other"), Some(Duration::from_secs(1800)));
    }
}
