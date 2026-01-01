# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2025-12-31

### Added

- Production-ready pre-release with comprehensive documentation
- Jekyll-based documentation site
- Complete examples including production-grade actixsqlx integration
- Comprehensive test suite with 193 test functions (~4000 lines)
- CI/CD pipeline with code coverage
- Docker Compose infrastructure for development and testing
- Makefile automation for common tasks
- **New:** Per-operation configuration with `OperationConfig` and `OperationBuilder` for TTL overrides and retry logic

### Changed

- Version bumped to 0.9.0 for production pre-release
- Documentation reorganized for better navigation
- Performance optimizations and stability improvements

## [0.2.0] - 2025-12-27

### Changed

- **Fixed lock contention issue:** `CacheBackend` and `CacheExpander` now use `&self` instead of `&mut self`, eliminating need for external `Arc<Mutex<>>` wrappers
- Added `CacheService<B>` wrapper for easier integration with web frameworks

## [0.1.0] - Dec 25, 2025

### Added

- Initial release of cache-kit, a type-safe, fully generic caching framework for Rust
- Fully generic caching over any Rust type `<T>` implementing `CacheEntity`
- Backend support: InMemory (default), Redis, and Memcached (feature-gated)
- Core traits: `CacheEntity<T>`, `CacheFeed<T>`, `DataRepository<T>`, and `CacheBackend`
- Cache strategies: `Fresh`, `Refresh` (default), `Invalidate`, and `Bypass`
- Observability: logging via `log` crate, `CacheMetrics` trait, and `TtlPolicy` enum
- Type-safe cache key generation with `CacheKeyBuilder` and `KeyRegistry`
- Generic implementations: `GenericFeeder<T>`, `CollectionFeeder<T>`, and `InMemoryRepository<T>`
- Comprehensive error handling with `Error` enum
- Thread-safe design: all components are `Send + Sync`
- Feature flags: `inmemory`, `redis`, `memcached`, `all`
- Complete documentation with examples and guides
- 53 unit tests with ~65% code coverage

### Changed

- None (initial release)

### Fixed

- None (initial release)

### Removed

- None (initial release)
