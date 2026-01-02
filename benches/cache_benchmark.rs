//! Comprehensive performance benchmarks for cache-kit
//!
//! This benchmark suite measures:
//! - InMemory backend operations (set, get, delete)
//! - CacheExpander operations (refresh hit/miss)
//! - Performance across different payload sizes
//!
//! Run with: cargo bench
//! View results: open target/criterion/report/index.html

use cache_kit::backend::{CacheBackend, InMemoryBackend};
use cache_kit::strategy::CacheStrategy;
use cache_kit::{CacheEntity, CacheExpander, CacheFeed, DataRepository};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};
use std::hint::black_box;

// ============================================================================
// Benchmark Test Fixtures
// ============================================================================

/// Benchmark entity with configurable data size
#[derive(Clone, Serialize, Deserialize)]
struct BenchEntity {
    id: String,
    data: Vec<u8>,
}

impl CacheEntity for BenchEntity {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.id.clone()
    }

    fn cache_prefix() -> &'static str {
        "bench"
    }
}

impl BenchEntity {
    fn new(id: String, size: usize) -> Self {
        BenchEntity {
            id,
            data: vec![0u8; size],
        }
    }
}

/// Feeder for benchmark entities
struct BenchFeeder {
    id: String,
    entity: Option<BenchEntity>,
}

impl BenchFeeder {
    fn new(id: String) -> Self {
        BenchFeeder { id, entity: None }
    }
}

impl CacheFeed<BenchEntity> for BenchFeeder {
    fn entity_id(&mut self) -> String {
        self.id.clone()
    }

    fn feed(&mut self, entity: Option<BenchEntity>) {
        self.entity = entity;
    }
}

/// Simple in-memory repository for benchmarks
#[derive(Clone)]
struct BenchRepository {
    default_size: usize,
}

impl BenchRepository {
    fn new(default_size: usize) -> Self {
        BenchRepository { default_size }
    }
}

impl DataRepository<BenchEntity> for BenchRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<BenchEntity>> {
        Ok(Some(BenchEntity::new(id.clone(), self.default_size)))
    }
}

// ============================================================================
// Group 1: InMemory Backend Benchmarks
// ============================================================================

fn inmemory_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("inmemory_backend");

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // Benchmark different payload sizes
    for size in [100, 1_000, 10_000, 100_000].iter() {
        // SET operation
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("set", size), size, |b, &size| {
                let backend = InMemoryBackend::new();
                let value = vec![1u8; size];

                b.to_async(&rt).iter(|| async {
                    backend
                        .set(black_box("test_key"), black_box(value.clone()), None)
                        .await
                        .expect("Failed to set")
                });
            });

        // GET operation (cache hit)
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("get_hit", size), size, |b, &size| {
                let backend = InMemoryBackend::new();
                let value = vec![1u8; size];
                rt.block_on(async {
                    backend
                        .set("test_key", value, None)
                        .await
                        .expect("Failed to set");
                });

                b.to_async(&rt)
                    .iter(|| async { backend.get(black_box("test_key")).await });
            });
    }

    // GET operation (cache miss) - size doesn't matter for misses
    group.bench_function("get_miss", |b| {
        let backend = InMemoryBackend::new();

        b.to_async(&rt)
            .iter(|| async { backend.get(black_box("nonexistent_key")).await });
    });

    // DELETE operation
    group.bench_function("delete", |b| {
        let backend = InMemoryBackend::new();
        let value = vec![1u8; 1000];

        b.to_async(&rt).iter(|| async {
            // Setup: insert before each iteration
            backend
                .set("test_key", value.clone(), None)
                .await
                .expect("Failed to set");
            // Measure: delete operation
            backend.delete(black_box("test_key")).await
        });
    });

    // EXISTS operation
    group.bench_function("exists", |b| {
        let backend = InMemoryBackend::new();
        rt.block_on(async {
            backend
                .set("test_key", vec![1u8; 1000], None)
                .await
                .expect("Failed to set");
        });

        b.to_async(&rt)
            .iter(|| async { backend.exists(black_box("test_key")).await });
    });

    group.finish();
}

// ============================================================================
// Group 2: CacheExpander Benchmarks
// ============================================================================

fn expander_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_expander");

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // Benchmark different payload sizes for expander operations
    for size in [100, 1_000, 10_000].iter() {
        // Refresh strategy - CACHE HIT
        // Measures: cache lookup + deserialization + feed
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("refresh_hit", size), size, |b, &size| {
                let backend = InMemoryBackend::new();
                let expander = CacheExpander::new(backend);
                let repo = BenchRepository::new(size);

                // Pre-populate cache
                rt.block_on(async {
                    let mut setup_feeder = BenchFeeder::new("bench_hit".to_string());
                    expander
                        .with(&mut setup_feeder, &repo, CacheStrategy::Refresh)
                        .await
                        .expect("Failed to populate cache");
                });

                b.to_async(&rt).iter(|| async {
                    let mut feeder = BenchFeeder::new("bench_hit".to_string());
                    expander
                        .with(
                            black_box(&mut feeder),
                            black_box(&repo),
                            black_box(CacheStrategy::Refresh),
                        )
                        .await
                });
            });

        // Refresh strategy - CACHE MISS
        // Measures: cache lookup + DB fetch + serialization + cache store + feed
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("refresh_miss", size), size, |b, &size| {
                let backend = InMemoryBackend::new();
                let expander = std::sync::Arc::new(CacheExpander::new(backend));
                let repo = BenchRepository::new(size);

                let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
                b.to_async(&rt).iter(|| {
                    let counter = counter.clone();
                    let expander = expander.clone();
                    let repo = repo.clone();
                    async move {
                        // Use unique key for each iteration to force cache miss
                        let current = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let mut feeder = BenchFeeder::new(format!("bench_miss_{}", current));

                        expander
                            .with(
                                black_box(&mut feeder),
                                black_box(&repo),
                                black_box(CacheStrategy::Refresh),
                            )
                            .await
                    }
                });
            });
    }

    // Invalidate strategy
    // Measures: delete + DB fetch + cache store
    group.bench_function("invalidate", |b| {
        let backend = InMemoryBackend::new();
        let expander = CacheExpander::new(backend);
        let repo = BenchRepository::new(1000);

        // Pre-populate cache
        rt.block_on(async {
            let mut setup_feeder = BenchFeeder::new("bench_invalidate".to_string());
            expander
                .with(&mut setup_feeder, &repo, CacheStrategy::Refresh)
                .await
                .expect("Failed to populate cache");
        });

        b.to_async(&rt).iter(|| async {
            let mut feeder = BenchFeeder::new("bench_invalidate".to_string());
            expander
                .with(
                    black_box(&mut feeder),
                    black_box(&repo),
                    black_box(CacheStrategy::Invalidate),
                )
                .await
        });
    });

    // Bypass strategy
    // Measures: DB fetch + cache store (skip cache read)
    group.bench_function("bypass", |b| {
        let backend = InMemoryBackend::new();
        let expander = CacheExpander::new(backend);
        let repo = BenchRepository::new(1000);

        b.to_async(&rt).iter(|| async {
            let mut feeder = BenchFeeder::new("bench_bypass".to_string());
            expander
                .with(
                    black_box(&mut feeder),
                    black_box(&repo),
                    black_box(CacheStrategy::Bypass),
                )
                .await
        });
    });

    // Fresh strategy - cache hit
    group.bench_function("fresh_hit", |b| {
        let backend = InMemoryBackend::new();
        let expander = CacheExpander::new(backend);
        let repo = BenchRepository::new(1000);

        // Pre-populate cache
        rt.block_on(async {
            let mut setup_feeder = BenchFeeder::new("bench_fresh".to_string());
            expander
                .with(&mut setup_feeder, &repo, CacheStrategy::Refresh)
                .await
                .expect("Failed to populate cache");
        });

        b.to_async(&rt).iter(|| async {
            let mut feeder = BenchFeeder::new("bench_fresh".to_string());
            expander
                .with(
                    black_box(&mut feeder),
                    black_box(&repo),
                    black_box(CacheStrategy::Fresh),
                )
                .await
        });
    });

    group.finish();
}

// ============================================================================
// Group 3: Serialization Benchmarks
// ============================================================================

fn serialization_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        let entity = BenchEntity::new("test_id".to_string(), *size);

        // Serialize (Postcard with envelope)
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(BenchmarkId::new("serialize", size), &entity, |b, entity| {
                b.iter(|| entity.serialize_for_cache());
            });

        // Deserialize (Postcard with envelope)
        let serialized = entity.serialize_for_cache().unwrap();
        group
            .throughput(Throughput::Bytes(*size as u64))
            .bench_with_input(
                BenchmarkId::new("deserialize", size),
                &serialized,
                |b, serialized| {
                    b.iter(|| BenchEntity::deserialize_from_cache(black_box(serialized)));
                },
            );
    }

    group.finish();
}

// ============================================================================
// Benchmark Registration
// ============================================================================

criterion_group!(
    benches,
    inmemory_benchmarks,
    expander_benchmarks,
    serialization_benchmarks
);
criterion_main!(benches);
