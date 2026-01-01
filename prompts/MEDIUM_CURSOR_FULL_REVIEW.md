# Architectural Review: cache-kit API Design

## Executive Summary

After reviewing the codebase, I **partially agree** with your hypothesis, but with important caveats. The current design has some legitimate concerns, but promoting `expander.rs` as the primary API surface is already largely correct—the issue is that `builder.rs` adds unnecessary complexity and `service.rs` doesn't fully participate in the execution flow.

## 1. Validation of Concerns

### ✅ Valid Concerns

1. **Builder is underused**: Analysis shows:

   - All examples use `expander.with()` directly
   - Only tests use `builder()` pattern
   - `CacheService` doesn't expose builder functionality
   - This suggests the builder pattern adds complexity without proportional value

2. **Service feels thin**: `CacheService` is indeed just a thin `Arc` wrapper:

   - Only provides `execute()` which delegates to `expander.with()`
   - Doesn't expose `builder()` (users must access `expander()` first)
   - No additional functionality beyond thread-safety

3. **Execution flow confusion**:
   - `builder.execute()` → `builder.execute_with_retry()` → `expander.with()`
   - This creates an unnecessary indirection layer
   - TTL override logic in builder mutates expander state temporarily (code smell)

### ❌ Challenged Concerns

1. **"expander.rs appears underused"**: **This is incorrect**. Analysis shows:

   - `expander.with()` is the primary execution path
   - All examples use it directly
   - It contains the core strategy execution logic
   - It's the most important API surface

2. **"expander doesn't feel like first-class API"**: **Disagreed**. It's exported in `lib.rs` and used throughout. The issue is the builder layer adds confusion.

## 2. API Design Review

### Current Mental Model

```
User → CacheService/Expander → Builder (optional) → Expander.with() → Strategy execution
```

### Recommended Mental Model

```
Simple path:   User → Expander.with() → Strategy execution
Complex path:  User → Expander.configure().execute() → Strategy execution
```

### What Should Users Interact With?

**Primary API: `CacheExpander`**

- This is already correct and well-designed
- `with()` method is clean, async-first, and ergonomic
- Configuration methods (`with_metrics`, `with_ttl_policy`) are appropriate

**Secondary API: `CacheService`**

- Valid for thread-safe sharing (Arc wrapper)
- Should mirror expander's API more closely
- Consider removing it or making it a thin convenience wrapper

**Tertiary API: Builder pattern**

- Current implementation is problematic:
  - Requires `&mut self` on expander (defeats purpose of Arc sharing)
  - TTL override mutates expander state temporarily
  - Retry logic could be better integrated
  - Not exposed through `CacheService`

## 3. Responsibility Boundaries

### Current State Analysis

**`expander.rs`** (Current responsibilities)

- ✅ Core cache strategy execution (Fresh, Refresh, Invalidate, Bypass)
- ✅ TTL policy management (long-lived configuration)
- ✅ Metrics integration
- ✅ Backend interaction
- ✅ Entity deserialization and validation
- ⚠️ Configuration methods (`with_metrics`, `with_ttl_policy`)
- ✅ Primary execution method (`with()`)

**`builder.rs`** (Current responsibilities)

- ✅ Operation-level configuration (strategy, TTL override, retry)
- ⚠️ Execution coordination
- ⚠️ TTL override via temporary state mutation
- ⚠️ Retry logic with exponential backoff

**`service.rs`** (Current responsibilities)

- ✅ Thread-safe wrapper (Arc)
- ✅ Basic execution delegation
- ❌ Doesn't expose builder functionality
- ❌ Doesn't participate in execution flow

### Recommended Boundaries

**`expander.rs`** - Core Execution Engine

- **Keep**: All strategy execution logic
- **Keep**: Long-lived configuration (metrics, TTL policy)
- **Keep**: Primary execution method (`with()`)
- **Add**: Operation-level configuration builder (integrated, not separate)
- **Remove**: Nothing—this is the core

**`builder.rs`** - Operation Configuration (Refactored)

- **Move to expander**: Configuration state storage
- **Move to expander**: Execution boundary
- **Keep here or move**: Retry logic (could be strategy-specific or expander-level)
- **Eliminate**: Temporary state mutation pattern

**`service.rs`** - Thread-Safe Convenience Wrapper

- **Keep**: Arc wrapper for sharing
- **Enhance**: Mirror expander's full API
- **Add**: Builder access through service
- **Clarify**: It's a convenience, not a requirement

## 4. Proposed Refactor Plan

### Phase 1: Consolidate Configuration in Expander (Non-Breaking)

**Goal**: Move operation-level configuration into `CacheExpander` without breaking existing API.

1. **Add operation configuration struct to expander.rs**:

   ```rust
   pub struct OperationConfig {
       strategy: CacheStrategy,
       ttl_override: Option<Duration>,
       retry_count: u32,
   }
   ```

2. **Add fluent configuration methods to CacheExpander**:

   ```rust
   impl<B: CacheBackend> CacheExpander<B> {
       // Keep existing methods unchanged

       // Add new fluent configuration
       pub fn configure(&mut self) -> OperationConfigBuilder<B> {
           OperationConfigBuilder::new(self)
       }
   }
   ```

3. **Create internal OperationConfigBuilder** (similar to current builder but better integrated):

   - Stores config, doesn't mutate expander state
   - `execute()` method that takes config and executes

4. **Update builder.rs** to use new pattern internally
   - Keep existing API temporarily (deprecate)
   - Internally use new expander configuration

### Phase 2: Improve Service API (Minor Breaking Changes)

1. **Add builder access to CacheService**:

   ```rust
   impl<B: CacheBackend> CacheService<B> {
       pub fn configure(&self) -> OperationConfigBuilder<B> {
           // Need to handle Arc + mutability
           // Could use interior mutability or different pattern
       }
   }
   ```

2. **Problem**: Arc + mutability conflict
   - Option A: Use `Arc<Mutex<CacheExpander<B>>>` (adds overhead)
   - Option B: Make configuration methods take `&self` and return owned config
   - Option C: Remove builder from service, recommend expander for advanced cases

### Phase 3: Simplify Execution Path (Breaking Changes)

1. **Remove temporary state mutation**:

   - Current: Builder mutates `expander.ttl_policy`, executes, restores
   - New: Pass TTL override as parameter to execution method
   - Store override in `OperationConfig`, pass to strategy methods

2. **Integrate retry logic better**:
   - Option A: Make retry a strategy-level concern
   - Option B: Make retry a configuration option passed to execution
   - Option C: Keep as expander-level concern but without builder indirection

### Phase 4: Deprecate Old Builder (Breaking Changes)

1. **Deprecate `builder()` method** on `CacheExpander`
2. **Add migration guide** showing old vs new patterns
3. **Remove in next major version**

## 5. Rust Idiomatic Considerations

### Ownership & Lifetimes

**Current Issues**:

- Builder requires `&mut self` on expander, preventing Arc sharing
- Temporary state mutation (TTL override) is not idiomatic Rust
- Lifetime complexity with builder pattern

**Recommended Approach**:

```rust
// Option 1: Owned configuration (recommended)
pub struct OperationConfig {
    strategy: CacheStrategy,
    ttl_override: Option<Duration>,
    retry_count: u32,
}

impl<B: CacheBackend> CacheExpander<B> {
    pub fn with_config<T, F, R>(
        &self,  // Note: &self, not &mut self
        feeder: &mut F,
        repository: &R,
        config: OperationConfig,
    ) -> Result<()> { ... }
}

// Usage:
let config = OperationConfig::default()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .with_retry(3);
expander.with_config(&mut feeder, &repo, config).await?;
```

**Benefits**:

- `&self` methods enable Arc sharing
- No temporary state mutation
- Clearer ownership semantics
- Better testability

### Fluent Builders vs Stateful Expanders

**Current**: Hybrid approach (confusing)

- Expander has `with_*` methods for long-lived config
- Builder has `with_*` methods for operation config
- Unclear which to use when

**Recommended**: Clear separation

- Expander `with_*` methods: Long-lived configuration (metrics, TTL policy)
- Operation config: Separate builder or struct with builder methods
- Clear documentation on when to use each

### Async Ergonomics

**Current**: Good

- All async methods are well-designed
- Proper use of `async fn` and `await`
- No blocking operations

**Recommendation**: Keep as-is, but simplify execution paths

### Testability

**Current**: Good test coverage

- All components are testable
- Good use of traits for dependency injection

**Recommendation**:

- Moving away from `&mut self` requirements improves testability
- Owned configuration structs are easier to test in isolation

## 6. Final Recommendation

### ✅ Agree With Your Direction, With Modifications

**Core Insight**: The builder pattern as currently implemented adds complexity without sufficient value. However, operation-level configuration (TTL override, retry) is still valuable.

### Recommended Approach: Integrated Configuration Builder

Instead of promoting expander as the _only_ API and removing builder entirely, I recommend:

1. **Keep `CacheExpander` as primary API** ✅ (already correct)
2. **Add operation configuration to expander** (not separate builder)
3. **Make configuration methods work with `&self`** (enables Arc sharing)
4. **Simplify `CacheService`** to be a thin convenience wrapper
5. **Deprecate current `builder()` pattern** (too complex, requires mutability)

### Concrete Implementation Pattern

```rust
// Long-lived configuration (expander-level)
let expander = CacheExpander::new(backend)
    .with_metrics(metrics)
    .with_ttl_policy(ttl_policy);

// Operation-level configuration (per-request)
let op_config = OperationConfig::default()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl_override(Duration::from_secs(300))
    .with_retry(3);

expander.with_config(&mut feeder, &repo, op_config).await?;

// Or, for simple cases (no operation config needed):
expander.with(&mut feeder, &repo, CacheStrategy::Refresh).await?;
```

### What NOT to Do

❌ Don't remove the builder pattern entirely—operation-level config is valuable
❌ Don't require `&mut self` for configuration—breaks Arc sharing
❌ Don't mutate expander state temporarily—not idiomatic Rust
❌ Don't make service.rs do heavy lifting—keep it thin

### Migration Path

1. **Phase 1** (Non-breaking): Add new configuration API alongside existing
2. **Phase 2** (Deprecation): Mark old builder as deprecated
3. **Phase 3** (Breaking): Remove old builder, require migration

This allows users to migrate gradually and maintains API stability.

## Summary

Your intuition about the design issues is correct, but the solution isn't to remove the builder entirely—it's to integrate operation-level configuration more cleanly into the expander API, using owned configuration structs instead of requiring mutability. This maintains the benefits of the builder pattern (fluent API, composability) while fixing the ownership and complexity issues.

---

## FINAL RECOMMENDATION

### Decision: Decouple Builder from Expander Ownership, Preserve Fluent API

After synthesizing all three architectural reviews, the unified verdict is to **fix the builder pattern by eliminating the `&mut CacheExpander` requirement**, while preserving fluent ergonomics for users who want them. This approach synthesizes the best aspects of each review:

**From Cursor & AMP**: Keep fluent builder pattern for ergonomic operation-level configuration  
**From CC**: Recognize that explicit config structs are also valuable and should be directly accessible  
**From All**: Eliminate temporary state mutation and Arc incompatibility

### Core Architectural Changes

1. **Introduce `CacheConfig` as a first-class value type** (not builder-held state):

   ```rust
   #[derive(Clone, Debug)]
   pub struct CacheConfig {
       strategy: CacheStrategy,
       ttl_override: Option<Duration>,
       retry_count: u32,
   }

   impl CacheConfig {
       pub fn default() -> Self { ... }
       pub fn with_strategy(mut self, s: CacheStrategy) -> Self { ... }
       pub fn with_ttl(mut self, ttl: Duration) -> Self { ... }
       pub fn with_retry(mut self, count: u32) -> Self { ... }
   }
   ```

2. **Refactor `CacheOperationBuilder` to be stateless**:

   - Builder no longer holds `&'a mut CacheExpander`
   - Builder accumulates config and returns owned `CacheConfig`
   - Terminal method `execute()` takes `&CacheExpander` explicitly as parameter
   - This eliminates all ownership conflicts with `Arc`

3. **Add `execute_with_config()` to both `CacheExpander` and `CacheService`**:
   - Direct method accepting `CacheConfig` struct
   - Works with `&self`, fully Arc-compatible
   - Passes TTL override as parameter (no state mutation)

### Final API Shape

**Simple path (unchanged):**

```rust
expander.with(&mut feeder, &repo, CacheStrategy::Refresh).await?;
service.execute(&mut feeder, &repo, CacheStrategy::Refresh).await?;
```

**Explicit config (CC's preference, also supported):**

```rust
let config = CacheConfig::default()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .with_retry(3);
expander.execute_with_config(&mut feeder, &repo, config).await?;
service.execute_with_config(&mut feeder, &repo, config).await?;
```

**Fluent builder (Cursor/AMP's preference, now Arc-compatible):**

```rust
// Works through Arc now!
service.builder()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .with_retry(3)
    .execute(&mut feeder, &repo)  // expander passed internally
    .await?;
```

### Why This Synthesis Works

1. **Fixes ownership issues**: Builder is stateless, config is owned value, works with `Arc<CacheExpander>`
2. **Preserves ergonomics**: Fluent API available for users who prefer it
3. **Provides explicitness**: Direct config struct for users who prefer explicit patterns
4. **Maintains single execution path**: Both builder and explicit config call the same `execute_with_config()` method
5. **Eliminates mutation**: TTL override passed as parameter, not mutated on expander

### Responsibility Boundaries (Final)

**`expander.rs`** - Primary execution engine:

- Core strategy execution (`with()`, `execute_with_config()`)
- Long-lived configuration (`with_metrics()`, `with_ttl_policy()`)
- No builder implementation (moved to builder.rs)

**`builder.rs`** - Stateless configuration builder:

- `CacheConfig` struct definition
- `CacheOperationBuilder` as stateless fluent API
- Returns owned `CacheConfig`, does not hold expander references
- Works with `&CacheExpander` (not `&mut`)

**`service.rs`** - Thread-safe convenience wrapper:

- Mirrors expander's API (`execute()`, `execute_with_config()`, `builder()`)
- All methods work with `&self`, fully Arc-compatible
- No special cases or escape hatches needed

### Migration Path

1. **Phase 1** (Non-breaking): Add `CacheConfig` and `execute_with_config()` alongside existing API
2. **Phase 2** (Deprecation): Refactor builder to new pattern, mark old pattern deprecated
3. **Phase 3** (Breaking): Remove deprecated builder pattern in next major version

### Breaking Change Impact

**Low risk**: Current `builder()` is `pub(crate)` and only used in tests. The new builder API will be public and work identically from user perspective, just with fixed ownership semantics internally.

**This design is maintainable, idiomatic, fixes all ownership issues, and provides both explicit and fluent APIs for different user preferences.**
