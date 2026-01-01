//! Cache strategies and decision logic for fetch operations.
//!
//! This module defines the different strategies for accessing cached data and provides
//! contextual information about cache operations.
//!
//! # Overview
//!
//! Cache-kit uses an enum-based strategy pattern to replace ad-hoc boolean flags.
//! This makes cache behavior explicit and type-safe.
//!
//! # The Four Strategies
//!
//! Every cache operation uses one of four strategies:
//!
//! ```
//! use cache_kit::strategy::CacheStrategy;
//!
//! // 1. Fresh - Use cache only
//! let _s = CacheStrategy::Fresh;
//!
//! // 2. Refresh - Cache-first with DB fallback (default)
//! let _s = CacheStrategy::Refresh;
//!
//! // 3. Invalidate - Clear cache, always fetch fresh
//! let _s = CacheStrategy::Invalidate;
//!
//! // 4. Bypass - Skip cache entirely
//! let _s = CacheStrategy::Bypass;
//! ```
//!
//! # Decision Tree
//!
//! ```text
//! Do you have data in cache?
//!     ├─ YES, and you trust it
//!     │  └─ Use: Fresh or Refresh
//!     │
//!     ├─ NO, or it's possibly stale
//!     │  └─ Use: Invalidate or Refresh
//!     │
//!     └─ You don't want caching now
//!        └─ Use: Bypass
//! ```
//!
//! # When to Use Each Strategy
//!
//! | Strategy | Cache Hit | Cache Miss | Use Case |
//! |----------|-----------|-----------|----------|
//! | **Fresh** | Return | Return None | Assume data cached; miss is error |
//! | **Refresh** | Return | DB fallback | Default; prefer cache, ensure availability |
//! | **Invalidate** | Delete | Fetch DB | After mutations; need fresh data |
//! | **Bypass** | Ignore | DB always | Testing or temporary disable |
//!
//! # Examples by Scenario
//!
//! These examples show typical usage patterns for different strategies.
//! See the module tests for complete runnable examples.
//!
//! # Trade-offs
//!
//! - **Refresh** (default): Best for most cases. Balances performance and consistency.
//! - **Fresh**: Fastest if hit, but fails on cache miss.
//! - **Invalidate**: Ensures freshness but increases DB load after mutations.
//! - **Bypass**: Simplest for testing, but defeats caching benefits.

use std::time::Duration;

/// Strategy enum controlling cache fetch/invalidation behavior.
///
/// Replaces boolean flags with explicit, type-safe options.
///
/// # Examples
///
/// ```
/// use cache_kit::strategy::CacheStrategy;
///
/// // Use cache only
/// let _strategy = CacheStrategy::Fresh;
///
/// // Try cache, fallback to database
/// let _strategy = CacheStrategy::Refresh;
///
/// // Clear cache and refresh from database
/// let _strategy = CacheStrategy::Invalidate;
///
/// // Skip cache entirely
/// let _strategy = CacheStrategy::Bypass;
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum CacheStrategy {
    /// **Fresh**: Try cache only, no fallback to database.
    ///
    /// Use when: You know data should be in cache, and miss is an error condition.
    ///
    /// Flow:
    /// 1. Check cache
    /// 2. If hit: return cached value
    /// 3. If miss: return None (don't hit database)
    Fresh,

    /// **Refresh**: Try cache first, fallback to database on miss.
    ///
    /// Use when: Default behavior, prefer cache but ensure data availability.
    ///
    /// Flow:
    /// 1. Check cache
    /// 2. If hit: return cached value
    /// 3. If miss: fetch from database
    /// 4. Store in cache
    /// 5. Return value
    #[default]
    Refresh,

    /// **Invalidate**: Mark cache as invalid and refresh from database.
    ///
    /// Use when: You know cache is stale and need fresh data.
    /// Typical use: After update/mutation operations.
    ///
    /// Flow:
    /// 1. Delete from cache
    /// 2. Fetch from database
    /// 3. Store in cache
    /// 4. Return value
    Invalidate,

    /// **Bypass**: Ignore cache entirely, always fetch from database.
    ///
    /// Use when: Cache is temporarily disabled or for specific read-through scenarios.
    ///
    /// Flow:
    /// 1. Fetch from database
    /// 2. Store in cache (for others)
    /// 3. Return value
    Bypass,
}

impl std::fmt::Display for CacheStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheStrategy::Fresh => write!(f, "Fresh"),
            CacheStrategy::Refresh => write!(f, "Refresh"),
            CacheStrategy::Invalidate => write!(f, "Invalidate"),
            CacheStrategy::Bypass => write!(f, "Bypass"),
        }
    }
}

/// Context information for cache operations.
#[derive(Clone, Debug)]
pub struct CacheContext {
    /// Cache key
    pub key: String,

    /// Whether value exists in cache
    pub is_cached: bool,

    /// Remaining TTL if cached
    pub ttl_remaining: Option<Duration>,

    /// Timestamp of last cache update
    pub cached_at: Option<std::time::Instant>,

    /// Custom metadata (user-provided)
    pub metadata: std::collections::HashMap<String, String>,
}

impl CacheContext {
    pub fn new(key: String) -> Self {
        CacheContext {
            key,
            is_cached: false,
            ttl_remaining: None,
            cached_at: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_cached(mut self, is_cached: bool) -> Self {
        self.is_cached = is_cached;
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl_remaining = Some(ttl);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_display() {
        assert_eq!(CacheStrategy::Fresh.to_string(), "Fresh");
        assert_eq!(CacheStrategy::Refresh.to_string(), "Refresh");
        assert_eq!(CacheStrategy::Invalidate.to_string(), "Invalidate");
        assert_eq!(CacheStrategy::Bypass.to_string(), "Bypass");
    }

    #[test]
    fn test_strategy_default() {
        assert_eq!(CacheStrategy::default(), CacheStrategy::Refresh);
    }

    #[test]
    fn test_strategy_equality() {
        assert_eq!(CacheStrategy::Fresh, CacheStrategy::Fresh);
        assert_ne!(CacheStrategy::Fresh, CacheStrategy::Refresh);
    }

    #[test]
    fn test_cache_context_builder() {
        let ctx = CacheContext::new("test_key".to_string())
            .with_cached(true)
            .with_ttl(Duration::from_secs(300));

        assert_eq!(ctx.key, "test_key");
        assert!(ctx.is_cached);
        assert_eq!(ctx.ttl_remaining, Some(Duration::from_secs(300)));
    }
}
