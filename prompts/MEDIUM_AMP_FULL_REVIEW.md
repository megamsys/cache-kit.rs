# Architecture Review: builder.rs, expander.rs, service.rs

**Status**: Detailed analysis and refactor recommendations  
**Scope**: Internal design of cache configuration and execution layers

---

## 1. Hypothesis Validation ✓ CONFIRMED WITH CAVEATS

Your hypothesis about `expander` being underused is **partially valid**, but the diagnosis needs refinement:

### What's Actually Happening

- `CacheExpander` is the **primary user-facing entry point** (used in examples, public API)
- `CacheOperationBuilder` exists as a **wrapper layer** for advanced configuration
- `CacheService` is a **convenience wrapper** for `Arc<CacheExpander>` but **bypasses builder entirely**
- The builder calls back to `expander.with()`, not the reverse

### The Real Problem

Responsibility is **inverted and layered confusingly**. The builder should not hold a `&'a mut CacheExpander` and mutate its TTL state. This creates a circular dependency in mental models:

- User calls `expander.builder()` → gets mutable reference to the same expander
- Builder mutates `expander.ttl_policy` 
- Builder calls `expander.with()` 
- After execution, TTL is restored

This is **fragile** (what if another thread holds the expander?). It also means:
- `CacheService` cannot use the builder safely (`Arc<CacheExpander>` + `&mut` = problem)
- TTL override via builder is a **mutation side effect**, not composition

---

## 2. API Design Review

### Current Shape Problems

| Layer | Role | Problem |
|-------|------|---------|
| `expander.with()` | Straight execution | ✓ Correct, but config happens elsewhere |
| `expander.builder()` | Config + execution | ✗ Mutable borrow; only works with direct expander access |
| `service.execute()` | Wrapper | ✗ Thin wrapper; doesn't expose builder |
| `service.expander()` | Escape hatch | ✗ Users must drop to `Arc<...>` and get mutable borrow |

### Mental Model Problem

A user must choose: "Do I use `expander.with()` directly, or `expander.builder()`?"

There's no clear path for `CacheService` users to do advanced config (retry, TTL override).

---

## 3. Responsibility Boundaries - PROPOSED

### Ideal Architecture

```
builder.rs
  ↓ (internal only)
expander.rs ← PRIMARY PUBLIC API
  ↓ (uses)
strategy.rs + backend + repository
  
service.rs ← HIGH-LEVEL WRAPPER (optional, app-specific)
  ↓ (wraps)
expander.rs
```

### Each File's Job

#### **builder.rs** → `pub(crate)` only

- Stateless configuration struct that holds `strategy`, `ttl_override`, `retry_count`
- Does **NOT** hold `&mut CacheExpander`
- Does **NOT** mutate expander state
- Executes via delegation to expander methods with explicit parameters
- Represents a **transaction of configuration**, not a reference to a mutable object

#### **expander.rs** → Primary API

- Core method: `expander.with(..., strategy)` for direct execution
- **New method**: `expander.execute(..., config)` where `config` contains retry, TTL, strategy
- `builder()` returns a **stateless config builder** (not self-referential)
- Optional: `with_metrics()`, `with_ttl_policy()` remain (they're setters, not config builders)
- Responsible for orchestrating strategies and managing the overall cache flow

#### **service.rs** → Arc wrapper + convenience layer

- Wraps `Arc<CacheExpander>`
- Exposes both: `service.execute()` (direct) and builder pattern through public API
- Used for: web apps, dependency injection, thread-safe sharing
- **Does not** need escape hatches like `.expander()` once builder is properly implemented

---

## 4. Rust Idiomatic Considerations

### Current Design Violations

1. **Borrowed state aliasing**: Builder holds `&'a mut` to expander, then calls `&self` methods on it
2. **Interior mutability pattern misuse**: TTL mutation through builder is a side effect, not a composable operation
3. **Arc incompatibility**: Builder pattern doesn't work with `CacheService`
4. **No separation of concerns**: Configuration and execution are entangled

### Better Approach

- **Configuration-as-value**: Build a `CacheConfig` struct (`strategy`, `ttl_override`, `retry_count`)
- **Fluent builder pattern**: Return `Self`, accumulate config, call a terminal `execute()` method that consumes the config
- **No mutation**: TTL override is a field in config, not a side effect on the expander

### Ownership Model: Before vs. After

**Current (problematic):**
```rust
let result = expander.builder()          // &mut CacheExpander
    .with_ttl(Duration::from_secs(300))  // mutates expander.ttl_policy
    .execute(&mut feeder, &repo)         // needs &mut F
    .await?;
```

**Proposed (idiomatic):**
```rust
let config = CacheConfig::default()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .with_retry(3);

expander.execute_with_config(&mut feeder, &repo, config).await?;
```

**Benefits:**
- No borrowing conflicts
- Works with `Arc<CacheExpander>`
- Config can be reused, tested independently
- TTL override is explicit in the config, not a hidden mutation

---

## 5. Refactor Plan (High-Level)

### Phase 1: Decouple Builder from Expander (PRIORITY)

**Goal**: Eliminate the `&'a mut CacheExpander` reference from builder.

```
1. Create CacheConfig (value type):
   pub struct CacheConfig {
       strategy: CacheStrategy,
       ttl_override: Option<Duration>,
       retry_count: u32,
   }
   
   impl CacheConfig {
       pub fn with_strategy(mut self, strategy: CacheStrategy) -> Self { ... }
       pub fn with_ttl(mut self, ttl: Duration) -> Self { ... }
       pub fn with_retry(mut self, count: u32) -> Self { ... }
   }

2. Refactor CacheOperationBuilder:
   pub struct CacheOperationBuilder {
       config: CacheConfig,
   }
   
   impl CacheOperationBuilder {
       pub fn new() -> Self { ... }
       pub fn with_strategy(mut self, strategy: CacheStrategy) -> Self { ... }
       pub fn with_ttl(mut self, ttl: Duration) -> Self { ... }
       pub fn with_retry(mut self, count: u32) -> Self { ... }
       
       // Terminal method: consumes config, executes via &self
       pub async fn execute<T, F, R>(
           self,
           expander: &CacheExpander<B>,
           feeder: &mut F,
           repository: &R,
       ) -> Result<()> { ... }
   }

3. Update CacheExpander:
   impl<B: CacheBackend> CacheExpander<B> {
       pub fn builder() -> CacheOperationBuilder { ... }
       
       pub async fn execute_with_config<T, F, R>(
           &self,
           feeder: &mut F,
           repository: &R,
           config: CacheConfig,
       ) -> Result<()> { ... }
   }
```

### Phase 2: Unify Public API (MEDIUM PRIORITY)

```
1. CacheExpander remains the primary entry point
   - .with(feeder, repo, strategy)         // direct execution
   - .execute_with_config(feeder, repo, config)  // advanced use
   - .builder()                            // stateless config builder

2. CacheService gains parity
   - .execute(feeder, repo, strategy)      // direct execution
   - .execute_with_config(feeder, repo, config)  // advanced use
   - .builder()                            // works with Arc<CacheExpander>
   - Consider deprecating .expander()
```

### Phase 3: Improve Config Composability (NICE TO HAVE)

```
1. Add #[derive(Clone)] to CacheConfig → reusable configs
   
2. Add convenience constructors:
   impl CacheConfig {
       pub fn fresh_read() -> Self { ... }
       pub fn write_through(ttl: Duration) -> Self { ... }
       pub fn invalidate_on_update() -> Self { ... }
   }

3. Consider profile presets for common patterns:
   - Read-heavy: high TTL, fresh strategy
   - Write-heavy: low TTL, invalidate strategy
   - Bypass: no caching
```

### Breaking Changes

- `expander.builder()` signature changes (returns value type, not `&mut` reference)
- Builder API surface changes (no longer has `execute()`, now builder is configured then passed to expander)
- `CacheService.expander()` may be deprecated or removed
- `CacheOperationBuilder` becomes public; implementation details may shift

**Migration path for users:**
```rust
// Old
expander.builder()
    .with_strategy(strategy)
    .with_ttl(ttl)
    .execute(&mut feeder, &repo)
    .await?;

// New
let config = CacheConfig::default()
    .with_strategy(strategy)
    .with_ttl(ttl);
expander.execute_with_config(&mut feeder, &repo, config).await?;

// Or, using builder ergonomics:
expander.builder()
    .with_strategy(strategy)
    .with_ttl(ttl)
    .execute(expander, &mut feeder, &repo)  // expander passed explicitly
    .await?;
```

---

## 6. Final Recommendation

### Implement the Refactor ✓

**Why this direction is correct:**

✅ **Correctness**: Eliminates aliasing and mutable borrow issues  
✅ **Ergonomics**: Works naturally with `Arc<CacheExpander>` (via `CacheService`)  
✅ **Clarity**: Configuration and execution are cleanly separated  
✅ **Scalability**: Easy to add new config options without API churn  
✅ **Testability**: `CacheConfig` can be tested independently  

### What the Current Design Gets Wrong

The current design is a **"builder-on-builder" anti-pattern** where:
- The builder mutates the object it came from
- This breaks composability 
- Makes the API unsafe to use from `CacheService`
- Violates Rust idioms around borrowing and ownership

### Recommended Sequencing

1. **Phase 1 First** (immediate): Move builder off the expander to eliminate mutual aliasing
   - This alone clarifies responsibilities and fixes the `CacheService` builder problem
   - No public API breakage if implemented carefully (can deprecate old builder() API)

2. **Phase 2** (after Phase 1 is solid): Expose builder and config through service API
   - Ensures all user-facing APIs are consistent

3. **Phase 3** (optional refinement): Add convenience constructors and presets
   - Improves ergonomics for common patterns
   - Can be added without further API churn

### What This Achieves

After refactoring, users will have a **clear mental model**:

```
1. Simple case: expander.with(&mut feeder, &repo, strategy).await?
2. Advanced case: let config = CacheConfig::default().with_retry(3)...
                  expander.execute_with_config(&mut feeder, &repo, config).await?
3. Fluent building: expander.builder().with_retry(3).execute(...)
4. Service-based: service.execute(&mut feeder, &repo, strategy).await?
                  service.execute_with_config(&mut feeder, &repo, config).await?
```

Each API surface is **independent, composable, and idiomatic**.

---

## FINAL RECOMMENDATION

### Decision: Implement the AMP Refactor Plan

After cross-referencing all three architectural reviews, I confirm the proposed refactor in this document (AMP) is the correct path forward. Here's the synthesis:

#### What Stays
- ✅ **`CacheExpander` as primary user-facing API** (Cursor & CC both confirm it's already correct)
- ✅ **`CacheService` as Arc-wrapper convenience layer** (necessary for web apps and DI patterns)
- ✅ **Operation-level configuration as a first-class capability** (TTL overrides and retry logic are legitimate, recurring needs)

#### What Changes
- **Decouple `CacheOperationBuilder` from `CacheExpander`**: Builder must not hold `&'a mut CacheExpander`. This is the root cause of all issues (incompatible with Arc, enables unsafe state mutation).
- **Introduce `CacheConfig` value type**: Immutable struct holding `strategy`, `ttl_override`, `retry_count`. This becomes the unit of per-operation configuration.
- **Fluent builder consumes config, not expander**: `CacheOperationBuilder::new()` returns a self-contained builder. Terminal method `execute()` takes `&CacheExpander` explicitly, not held as a reference.
- **Remove temporary state mutation pattern**: TTL override lives in config, passed down to strategy execution, not swapped into `expander.ttl_policy`.

#### Why This Beats the Alternatives
1. **vs. CC's Option A (remove builder)**: Fluent APIs are idiomatic Rust (see `tokio`, `reqwest`, `clap`) when designed correctly. Operation-level config is too common to force into verbose parameter passing. The issue isn't builders; it's broken ownership.
2. **vs. CC's Option B (fix builder with borrowed references)**: That pattern still carries lifetime complexity into user code and duplicates strategy logic. AMP's approach is cleaner: own the config, don't borrow the engine.
3. **vs. Cursor's approach**: Cursor proposes similar ownership fixes but leaves some ambiguity on how service integrates with builder. AMP explicitly addresses this: service gains parity with expander, using the same builder/config pattern.

#### Concrete API After Refactor
```rust
// Simple case (unchanged)
expander.with(&mut feeder, &repo, CacheStrategy::Refresh).await?;

// Advanced case (fixed ownership)
let config = CacheConfig::default()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .with_retry(3);
expander.execute_with_config(&mut feeder, &repo, config).await?;

// Fluent builder (idiomatic)
expander.builder()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .with_retry(3)
    .execute(&expander, &mut feeder, &repo)  // expander passed explicitly
    .await?;

// Through service (now works!)
service.builder()
    .with_strategy(CacheStrategy::Refresh)
    .with_ttl(Duration::from_secs(300))
    .execute(&mut feeder, &repo)
    .await?;
```

#### Implementation Sequencing
1. **Phase 1 (IMMEDIATE)**: Introduce `CacheConfig` struct and `execute_with_config()` method. Make builder take `&CacheExpander` (not `&mut`), eliminate self-aliasing.
2. **Phase 2 (MEDIUM TERM)**: Expose builder through `CacheService` API. Ensure all execution paths (expander, builder, service) are consistent.
3. **Phase 3 (NICE-TO-HAVE)**: Add convenience constructors and presets to `CacheConfig` for common patterns.

#### Breaking Changes Impact
- `expander.builder()` returns value type instead of self-reference (users must pass expander explicitly to `execute()`)
- `CacheService.expander()` may be deprecated once builder is public and working through service
- Tests using old builder pattern need mechanical updates (CLI-friendly refactor)

**This design is maintainable, idiomatic, and solves the Arc incompatibility without sacrificing ergonomics.**

---

## References

- **Current file responsibilities**: See `src/builder.rs`, `src/expander.rs`, `src/service.rs`
- **Public exports**: `src/lib.rs`
- **Test coverage**: Extensive tests exist for all three modules; refactor should preserve test semantics
- **Related architectural reviews**: `ARCHITECTURE_REVIEW_CURSOR.md`, `ARCHITECHTURE_REVIEW_CC.md` (cross-referenced for consensus)
