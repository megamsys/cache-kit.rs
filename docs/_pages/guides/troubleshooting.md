---
layout: single
title: Production Troubleshooting
parent: Guides
nav_order: 13
date: 2026-01-01
---

Diagnose and resolve cache-kit issues in production environments.

---

## Overview

This guide covers the most common cache-kit issues, how to diagnose them, and how to fix them.

### Diagnostic Workflow

```
Issue Observed
    ↓
Identify Category (Connection? Performance? Data?)
    ↓
Gather Logs & Metrics
    ↓
Check Backend Health
    ↓
Apply Fix
    ↓
Verify Resolution
```

### Tools You'll Need

```bash
# Redis diagnosis
redis-cli
redis-cli --latency
redis-cli --stat

# Memcached diagnosis
echo "stats" | nc localhost 11211
memcached-tool localhost:11211

# Application logs
grep "cache" app.log | grep "error"

# System metrics
top
vmstat
netstat
```

---

## Common Issues & Solutions

### Issue 1: Low Cache Hit Rate (< 30%)

**Symptoms:**

- Hit rate stuck at 10-20%
- Cache size not growing
- High database load despite caching

**Possible Causes:**

1. Keys are not being reused (different key each time)
2. TTL is too short (entries expire quickly)
3. Cache keys are non-deterministic
4. Cache is being cleared unexpectedly
5. New users/data not being cached

#### Diagnosis Steps

**1. Check hit rate and TTL**

```bash
# Check hit rate from metrics
# Check TTL: redis-cli TTL "user:123"
# -1 = no expiration (problem!), -2 = key doesn't exist
```

**2. Verify keys are deterministic**

```rust
// ✅ Good: Deterministic key
fn cache_key(&self) -> String { self.id.clone() }

// ❌ Bad: Non-deterministic (creates new key each time)
fn cache_key(&self) -> String {
    format!("{}:{}", self.id, SystemTime::now().timestamp())
}
```

**3. Verify using Refresh strategy (not Fresh)**

```rust
// ✅ Correct: Cache with DB fallback
expander.with(&mut feeder, &repo, CacheStrategy::Refresh)?;
```

#### Solutions

**1. Set appropriate TTL**

```rust
let expander = CacheExpander::builder()
    .with_backend(backend)
    .with_ttl(Duration::from_secs(3600))  // 1 hour
    .build();
```

**2. Ensure keys are deterministic** - Use entity ID, not timestamps or random values

**3. Use Refresh strategy** - `CacheStrategy::Refresh` (not `Fresh`) to allow DB fallback

### Issue 2: Backend Connection Timeouts

**Symptoms:**

- Request timeouts after N milliseconds
- "Connection refused" errors
- Pool exhaustion errors
- p99 latency spikes

**Possible Causes:**

1. Backend (Redis/Memcached) is down
2. Network connectivity issue
3. Connection pool size is too small
4. Timeout is set too aggressively

#### Diagnosis Steps

**1. Verify backend is running**

```bash
redis-cli ping  # Should return PONG
# Or: echo "stats" | nc localhost 11211  # For Memcached
```

**2. Check network and latency**

```bash
nc -zv localhost 6379  # Test connectivity
redis-cli --latency     # Should be < 1ms (good), > 10ms (slow)
```

**3. Check pool size and timeout config** - Look for "pool exhausted" errors in logs

#### Solutions

**1. Restart backend** - `docker restart redis_container` or `redis-cli shutdown && redis-server`

**2. Increase pool size** - Use formula: `(CPU_cores × 2) + 1`

```rust
let config = RedisConfig {
    pool_size: (num_cpus::get() * 2 + 1) as u32,
    ..Default::default()
};
```

**3. Increase timeout** - Set `connection_timeout: Duration::from_secs(10)` (not 1-2 seconds)

---

### Issue 3: High Memory Usage

**Symptoms:**

- Cache backend consuming GB of RAM
- OOM killer triggering
- Eviction errors from backend
- Request latency increasing

**Possible Causes:**

1. Entries are too large (whole objects)
2. TTL not set (entries never expire)
3. Too many unique keys (unbounded growth)
4. No eviction policy configured
5. Memory leak in application

#### Diagnosis Steps

**1. Check memory usage and key count**

```bash
redis-cli INFO memory    # Check used_memory_human vs maxmemory
redis-cli DBSIZE         # Total key count
redis-cli --bigkeys      # Find large entries (>512 bytes = problem)
```

**2. Check eviction policy**

```bash
redis-cli CONFIG GET maxmemory-policy
# Should be "allkeys-lru" (good), not "no-eviction" (bad)
```

#### Solutions

**1. Set TTL** - Ensure entries expire: `.with_ttl(Duration::from_secs(3600))`

**2. Reduce entry size** - Cache only needed fields, exclude large blobs (images, passwords)

**3. Configure eviction policy**

```bash
redis-cli CONFIG SET maxmemory 2gb
redis-cli CONFIG SET maxmemory-policy allkeys-lru
```

---

### Issue 4: Serialization Errors

**Symptoms:**

- "Serialization failed" errors
- "Version mismatch" errors
- "Invalid magic header" errors
- Some requests fail, others work

**Possible Causes:**

1. Entity type changed (schema mismatch)
2. Corrupted cache entry
3. Type contains unsupported fields (e.g., Decimal)
4. Different serialization formats

#### Diagnosis Steps

**1. Check error logs**

```bash
grep -i "serialization" app.log | head -20
```

**2. Check for unsupported types or schema changes**

```rust
// ❌ Problem: Decimal not supported
struct User { balance: rust_decimal::Decimal }

// ❌ Problem: Schema changed (added/removed fields)
// Old: struct User { id, name }
// New: struct User { id, name, email }  // Added field!
```

#### Solutions

**1. Clear affected entries**

```bash
redis-cli DEL "user:123"  # Or: redis-cli KEYS "user:*" | xargs redis-cli DEL
```

**2. Replace unsupported types** - Use `i64` for `Decimal` (store as cents), or use cache-specific DTOs

---

## Logging Setup

Enable debug logging for troubleshooting:

```bash
RUST_LOG=cache_kit=debug cargo run
```

Use structured logging with context:

```rust
info!(user_id = %user_id, cache_hit = hit, "Cache operation completed");
```

See [Monitoring Guide](monitoring) for detailed logging setup.

---

## Health Checks

Implement health checks to monitor cache availability:

```rust
async fn health_check(cache: &mut CacheExpander<RedisBackend>) -> Result<HealthStatus> {
    let start = Instant::now();
    match cache.health_check().await {
        Ok(true) => {
            let latency = start.elapsed();
            if latency > Duration::from_millis(100) {
                Ok(HealthStatus::Degraded)
            } else {
                Ok(HealthStatus::Healthy)
            }
        }
        _ => Ok(HealthStatus::Unhealthy),
    }
}
```

See [Monitoring Guide](monitoring) for detailed health check implementation.

---

## Production Troubleshooting Checklist

Use this checklist when issues occur:

### Cache Issues

- [ ] Is Redis/Memcached running? (`redis-cli ping`)
- [ ] Is network connectivity OK? (`nc -zv localhost 6379`)
- [ ] Are connection pool metrics available?
- [ ] What's the hit rate? (< 20% = investigate TTL/keys)
- [ ] Are there serialization errors? (check entity types)
- [ ] Is memory usage growing unbounded? (check TTL)
- [ ] Are cache keys deterministic? (check key generation)

### Network Issues

- [ ] Is backend reachable? (`netstat` or `ss`)
- [ ] What's the latency? (`redis-cli --latency`)
- [ ] Are there packet drops? (`netstat -s`)
- [ ] Is there network congestion?
- [ ] Did firewall rules change?

### Application Issues

- [ ] Are error logs being generated? (`grep error app.log`)
- [ ] Is the cache fallback code working?
- [ ] Are metrics being exported?
- [ ] Is the database under load?
- [ ] Did schema change recently?

### System Issues

- [ ] CPU usage normal? (`top`)
- [ ] Memory usage normal? (`free -h`)
- [ ] Disk space available? (`df -h`)
- [ ] System load average? (`uptime`)
- [ ] Are there OOM killings? (`dmesg | tail`)

---

## Error Handling Best Practices

**Always provide fallbacks** - Cache failures shouldn't break your application:

```rust
match cache.with(&mut feeder, &repo, CacheStrategy::Refresh) {
    Ok(_) => feeder.user,
    Err(Error::BackendError(_)) => repo.fetch_by_id(&user_id).ok().flatten(),  // DB fallback
    Err(Error::SerializationError(_)) => {
        cache.with(&mut feeder, &repo, CacheStrategy::Invalidate).ok();
        repo.fetch_by_id(&user_id).ok().flatten()  // Invalidate and refetch
    }
    Err(e) => {
        error!("Cache error: {}", e);
        repo.fetch_by_id(&user_id).ok().flatten()
    }
}
```

**Never panic on cache errors** - Use `.map_err()` or `match`, never `.expect()`

**Log with context, not sensitive data** - Log IDs only: `error!("Failed to cache user {}: {}", user.id, e)`

---

## Next Steps

- Set up [Monitoring and metrics](monitoring) to detect issues early
