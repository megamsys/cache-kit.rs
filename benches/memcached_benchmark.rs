//! Comprehensive performance benchmarks for Memcached backend
//!
//! This benchmark suite measures:
//! - Memcached backend operations (set, get, delete)
//! - Batch operations (mget, mdelete)
//! - Performance across different payload sizes
//! - Binary protocol performance
//!
//! Prerequisites:
//! - Memcached running on localhost:11211
//! - Run with: cargo bench --bench memcached_benchmark --features memcached
//! - View results: open target/criterion/report/index.html

#![cfg(feature = "memcached")]

use cache_kit::backend::{CacheBackend, MemcachedBackend};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

// ============================================================================
// Memcached Backend Setup
// ============================================================================

/// Create a Memcached backend for benchmarking.
///
/// Pool size can be configured via `MEMCACHED_POOL_SIZE` environment variable.
/// Defaults to 16 if not specified (optimal for 8-core systems).
/// Formula: (CPU cores × 2) + 1
///
/// Example: MEMCACHED_POOL_SIZE=32 cargo bench --bench memcached_benchmark --features memcached
async fn setup_memcached() -> MemcachedBackend {
    MemcachedBackend::from_server("localhost:11211".to_string())
        .await
        .expect(
            "Failed to connect to Memcached at localhost:11211. Make sure Memcached is running.",
        )
}

// ============================================================================
// Group 1: Memcached Basic Operations
// ============================================================================

fn memcached_basic_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcached_backend");
    group.sample_size(50); // Fewer samples due to network latency

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let backend = rt.block_on(async { setup_memcached().await });

    // Clear any existing data
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    // Benchmark different payload sizes
    for size in [100, 1_000, 10_000, 100_000].iter() {
        // SET operation
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("set", size), size, |b, &size| {
                let value = vec![1u8; size];

                b.to_async(&rt).iter(|| async {
                    backend
                        .set(
                            black_box("memcached_bench_key"),
                            black_box(value.clone()),
                            None,
                        )
                        .await
                        .expect("Failed to set")
                });
            });

        // GET operation (cache hit)
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("get_hit", size), size, |b, &size| {
                let value = vec![1u8; size];
                rt.block_on(async {
                    backend
                        .set("memcached_bench_key", value, None)
                        .await
                        .expect("Failed to set");
                });

                b.to_async(&rt).iter(|| async {
                    backend
                        .get(black_box("memcached_bench_key"))
                        .await
                        .expect("Failed to get")
                });
            });
    }

    // GET operation (cache miss) - size doesn't matter for misses
    group.bench_function("get_miss", |b| {
        b.to_async(&rt).iter(|| async {
            backend
                .get(black_box("nonexistent_key"))
                .await
                .expect("Failed to get")
        });
    });

    // DELETE operation
    group.bench_function("delete", |b| {
        let value = vec![1u8; 1000];

        b.to_async(&rt).iter(|| async {
            // Setup: insert before each iteration
            backend
                .set("memcached_bench_delete", value.clone(), None)
                .await
                .expect("Failed to set");
            // Measure: delete operation
            backend
                .delete(black_box("memcached_bench_delete"))
                .await
                .expect("Failed to delete")
        });
    });

    // EXISTS operation
    group.bench_function("exists", |b| {
        rt.block_on(async {
            backend
                .set("memcached_bench_exists", vec![1u8; 1000], None)
                .await
                .expect("Failed to set");
        });

        b.to_async(&rt).iter(|| async {
            backend
                .exists(black_box("memcached_bench_exists"))
                .await
                .expect("Failed to check exists")
        });
    });

    // Cleanup
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    group.finish();
}

// ============================================================================
// Group 2: Memcached Batch Operations
// ============================================================================

fn memcached_batch_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcached_batch_ops");
    group.sample_size(50); // Fewer samples due to network latency

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let backend = rt.block_on(async { setup_memcached().await });

    // Clear any existing data
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    // Benchmark MGET with different batch sizes and payload sizes
    for batch_size in [10, 50, 100].iter() {
        for payload_size in [100, 1_000, 10_000].iter() {
            let keys: Vec<String> = (0..*batch_size)
                .map(|i| format!("memcached_mget_key_{}", i))
                .collect();

            // Pre-populate keys
            let value = vec![1u8; *payload_size];
            rt.block_on(async {
                for key in &keys {
                    backend
                        .set(key, value.clone(), None)
                        .await
                        .expect("Failed to set");
                }
            });

            let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();

            group
                .throughput(Throughput::Bytes((*batch_size * *payload_size) as u64))
                .bench_with_input(
                    BenchmarkId::new(
                        "mget",
                        format!("batch_{}_size_{}", batch_size, payload_size),
                    ),
                    &key_refs,
                    |b, keys| {
                        b.to_async(&rt).iter(|| async {
                            backend.mget(black_box(keys)).await.expect("Failed to mget")
                        });
                    },
                );
        }
    }

    // Benchmark MDELETE with different batch sizes
    for batch_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("mdelete", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    // Setup: create keys
                    let keys: Vec<String> = (0..batch_size)
                        .map(|i| format!("memcached_mdelete_key_{}", i))
                        .collect();

                    for key in &keys {
                        backend
                            .set(key, vec![1u8; 100], None)
                            .await
                            .expect("Failed to set");
                    }

                    let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();

                    // Measure: delete all keys
                    backend
                        .mdelete(black_box(&key_refs))
                        .await
                        .expect("Failed to mdelete")
                });
            },
        );
    }

    // Cleanup
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    group.finish();
}

// ============================================================================
// Group 3: Memcached Binary Protocol Performance
// ============================================================================

fn memcached_protocol_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcached_protocol");
    group.sample_size(50);

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let backend = rt.block_on(async { setup_memcached().await });

    // Clear any existing data
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    // Benchmark rapid consecutive operations
    group.bench_function("rapid_set_get_delete", |b| {
        let value = vec![1u8; 1000];

        b.to_async(&rt).iter(|| async {
            let key = "memcached_protocol_test";
            backend
                .set(black_box(key), black_box(value.clone()), None)
                .await
                .expect("Failed to set");
            let _ = backend.get(black_box(key)).await.expect("Failed to get");
            backend
                .delete(black_box(key))
                .await
                .expect("Failed to delete");
        });
    });

    // Benchmark health check
    group.bench_function("health_check", |b| {
        b.to_async(&rt)
            .iter(|| async { backend.health_check().await.expect("Failed health check") });
    });

    // Cleanup
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    group.finish();
}

// ============================================================================
// Group 4: Memcached TTL Operations
// ============================================================================

fn memcached_ttl_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcached_ttl");
    group.sample_size(50);

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let backend = rt.block_on(async { setup_memcached().await });

    // Clear any existing data
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    // Benchmark SET with TTL
    for size in [100, 1_000, 10_000].iter() {
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("set_with_ttl", size), size, |b, &size| {
                let value = vec![1u8; size];
                let ttl = Some(std::time::Duration::from_secs(60));

                b.to_async(&rt).iter(|| async {
                    backend
                        .set(
                            black_box("memcached_ttl_key"),
                            black_box(value.clone()),
                            black_box(ttl),
                        )
                        .await
                        .expect("Failed to set with TTL")
                });
            });
    }

    // Cleanup
    rt.block_on(async { backend.clear_all().await })
        .expect("Failed to clear Memcached");

    group.finish();
}

// ============================================================================
// Benchmark Registration
// ============================================================================

criterion_group!(
    benches,
    memcached_basic_benchmarks,
    memcached_batch_benchmarks,
    memcached_protocol_benchmarks,
    memcached_ttl_benchmarks
);
criterion_main!(benches);
