<!--
SPEC AUTHORING RULES (load-bearing — do not delete):
- No time/effort/hour/day estimates anywhere in this spec.
- No effort columns, complexity ratings, percentage-complete, implementation dates.
- No assigned owners — use git history and handoff notes.
- Priority (P0/P1/P2) is the only sizing signal. Use Dependencies for sequencing.
- If a section below contradicts these rules, the rule wins — delete the section.
- See ~/Projects/dotfiles/docs/TEMPLATE.md for the canonical template.
-->

# M092_003: `SimpleCache<K, V>` — moka-parity entry point for cache-kit

**Prototype:** v0.9.2
**Milestone:** M092
**Workstream:** 003
**Date:** May 16, 2026
**Status:** PENDING
**Priority:** P0 — first-touch friction is the dominant adoption barrier vs moka. Today's `README.md` quick-start requires implementing five traits/structs (`CacheEntity`, `CacheFeed`, `DataRepository`, plus wiring `CacheExpander` + `CacheStrategy`) across ~50 lines before reading a single value. moka does the same in 3 lines with zero traits. Engineers comparing libraries at evaluation-time bounce at the trait list. M092_001 (error codes) and M092_002 (observability) are second-touch DX — they help users who already adopted. M092_003 fixes the bounce-at-README problem and is the highest-leverage v0.9.2 work for adoption.
**Categories:** API · DOCS · DX
**Batch:** B0 — runs first in v0.9.2 because the new README/quick-start authored here is the surface every other v0.9.2 work links to. M092_001 and M092_002 land independently and add their own pages under the new versioned tree.
**Branch:** {feat/m092-simple-cache — added when work begins}
**Depends on:** none in code. Soft-depends on M092_002 §7 for the versioned docs tree (`site/content/v0.9.2/`); if M092_003 lands first, it creates that directory and M092_002 §7 picks up where it left off.
**Provenance:** agent-generated (synthesized from M092_002's "before/after" analysis showing the actual entry-point gap, grounded in `README.md:50-110` and `examples/basic_usage.rs` (243 lines))

> Cache-kit's existing `CacheExpander` + `CacheEntity` + `DataRepository` + `CacheFeed` + `CacheStrategy` pattern is **not removed and not deprecated** by this spec. It remains cache-kit's actual differentiator vs moka — read-through-with-repository is genuinely useful for service-layer caching. The mistake was making it the *only* on-ramp. M092_003 adds a sibling `SimpleCache<K, V>` surface for the 80% of "I want a `HashMap` with eviction" use cases. Same library, two on-ramps. The full pattern stays available; the README's "Going further" section points to it.

**Canonical architecture:** N/A — no `docs/ARCHITECTURE.md`. Implementation surface is a new `src/simple.rs` module that wraps the existing `CacheExpander<B>` internally, plus README + quick-start rewrites.

---

## Implementing agent — read these first

1. `src/expander.rs` (908 lines) — the existing surface `SimpleCache` will wrap. Identify the minimum subset of `CacheExpander` methods needed for `get` / `insert` / `remove` / `clear` / `get_or_insert_with`. SimpleCache MUST NOT duplicate backend handling, serialization, or TTL logic; it delegates.
2. `src/backend/mod.rs` and `src/backend/inmemory.rs` — the `Backend` trait shape determines what `SimpleCache::new(backend)` accepts. The default constructor `SimpleCache::default()` must produce an `InMemoryBackend`-backed cache with no further config required.
3. `src/entity.rs` and `src/feed.rs` — these are the traits the user is escaping. Read enough to confirm `SimpleCache` does not require the user to implement either. `K` and `V` only need standard derives (`Serialize + DeserializeOwned + Clone + Send + Sync + 'static`).
4. `src/observability.rs` — `TtlPolicy::Fixed(Duration)` already exists and is the primitive `SimpleCache::with_ttl` wires to. Per-insert TTL is genuinely new and lands as a small extension on the `Backend` trait or a `set_with_ttl` method on `CacheExpander`.
5. `README.md` (current quick-start at lines 50-110) — this is the file most evaluators read first. The post-spec README's Quick Start MUST be ≤ 15 lines of Rust (target: 5 lines + 5 lines of imports + 3 lines of struct definition). The full `CacheExpander` pattern moves to a "Going further" section linking to `site/content/v0.9.2/concepts/expander-and-repository.mdx`.
6. `examples/basic_usage.rs` (243 lines) — needs a sibling `examples/simple_usage.rs` (target: ≤ 30 lines including imports + entity definition + `tokio::main`) that the README quick-start mirrors line-for-line.
7. moka's `Cache` (https://docs.rs/moka/latest/moka/future/struct.Cache.html) — read the public method list. `SimpleCache`'s method names should be cache-kit-native (the user picked `Result<>` returns, not panic-on-error) but the **operations** moka exposes (get, insert, invalidate, get_with) are the parity target. Don't add operations moka doesn't have; don't omit operations moka has.

---

## PR Intent & comprehension handshake

> The bridge from spec to the merged PR. Makes the agent confirm it understood intent *before* writing code.

- **PR title (eventual):** `feat(api): add SimpleCache<K,V> — moka-parity entry point`
- **Intent (one sentence):** A Rust engineer evaluating cache-kit reads ≤15 lines and implements zero traits to `insert`/`get`/`remove`, while the full `CacheExpander` repository pattern stays first-class behind a "Going further" link.
- **Handshake (agent fills at PLAN, before EXECUTE):** the implementing agent restates the intent in its own words and lists `ASSUMPTIONS I'M MAKING: …`. A mismatch between that restatement and the Intent above → STOP and reconcile before any edit.

---

## Applicable Rules

- **`docs/greptile-learnings/RULES.md`** — universal repo discipline.
- **Project rule source:** `RUST_GUIDELINES.txt` at repo root — read sections covering trait additivity, generic bounds, and zero-cost abstractions.
- **Semver discipline** — purely additive. `SimpleCache` is a new public type; no existing names move or change shape.
- **Simplicity-first invariant (set by this spec)** — every public method on `SimpleCache` must work without the user implementing any cache-kit trait. If a method requires the user to implement `CacheEntity` / `CacheFeed` / `DataRepository`, it doesn't belong on `SimpleCache` — it belongs on `CacheExpander`. This invariant is testable: a doc-test with a `User` struct that derives only `Serialize + Deserialize + Clone` must compile against every `SimpleCache` method.
- **README budget rule** — the post-spec `README.md` Quick Start section (between `## Quick Start` and the next `##`) is bounded at 50 source lines. Today it sits at ~60. Any growth of the simple surface that pushes the README past 50 needs the surface trimmed, not the budget raised. Enforced by `scripts/check-readme-quickstart-budget.sh` invoked from CI.

---

## Applicable Gates

> Which Action-Triggered Guards this PR WILL trip, and how each stays clean. Rules ≠ Gates: rules are knowledge to read; gates fire on edits.

| Gate | Fires? | Satisfaction strategy |
|------|--------|-----------------------|
| File & Function Length (≤350/≤50/≤70) | yes — new `src/simple.rs` | Bounded at 350 (Files Changed says so); `SimpleCache` only forwards to `inner: CacheExpander`, so methods stay short. |
| PUB / Struct-Shape | yes — `SimpleCache<K,V,B>` + 3 constructors + 7 methods | Additive; only new public name is `SimpleCache` (Invariant 7). Doc-comment every method with a working snippet; the thundering-herd gap is doc-stated on `get_or_insert_with`. |
| UFS (repeated/semantic literals) | yes — cache-key prefix / `Display` formatting | Named constants for any literal key affix; no inline magic strings. |
| CI / scripts | yes — new `scripts/check-readme-quickstart-budget.sh` + `.github/workflows/ci.yml` edit | Shell script lints clean; **CI/CD workflow edit is forbidden without explicit user approval — surface the `.github/workflows/ci.yml` change to Indy before committing it.** |
| LOGGING / SCHEMA / ZIG / UI / DESIGN TOKEN | no | No log sites, `*.sql`, `*.zig`, or UI component touched (docs MDX only). |

---

## Overview

**Goal (testable):** A Rust engineer evaluating cache-kit on docs.rs at 11pm reads ≤ 15 lines of Rust to do `insert` / `get` / `remove`, and zero traits implemented. The README's Quick Start matches `examples/simple_usage.rs` exactly. The full repository-pattern surface remains available behind a "Going further" link, not as the front door.

**Problem (concrete, with line counts):**

| Library | Lines to first cache read | Traits user implements | Structs user defines |
|---------|---------------------------|------------------------|----------------------|
| moka | 3 | 0 | 0 (just `Cache::new(10_000)`) |
| cache-kit today | ~50 | 3 (`CacheEntity`, `CacheFeed`, `DataRepository`) | 3 (entity + feeder + repo) |
| cache-kit after M092_003 | ≤ 5 | 0 | 1 (just the entity, with derives) |

**Why the existing surface costs adoption:**
- **Evaluation bounce:** README looks like a framework, not a library. Engineers comparing libraries on docs.rs at 11pm don't read past the trait list.
- **No incremental on-ramp:** can't ship a 5-line proof-of-concept and grow into the full pattern. It's all-or-nothing.
- **Mental-model mismatch:** "I want a `HashMap` with eviction" (moka) vs "I want a read-through-with-repository service-layer caching pattern" (cache-kit today). For most cache use cases, moka's mental model is right. cache-kit forces the heavier model on everyone.

**Solution summary:** `pub struct SimpleCache<K, V, B = InMemoryBackend>` with a moka-parity operation set (`get`, `insert`, `insert_with_ttl`, `remove`, `clear`, `get_or_insert_with`, `try_get_or_insert_with`), cache-kit-native `Result<>` return shapes, no required user traits beyond `Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Wraps `CacheExpander<B>` internally so all backend handling, serialization, and TTL logic stay in one place — no duplication, no drift. README + quick-start are rewritten to lead with `SimpleCache`.

---

## Prior-Art / Reference Implementations

> Mirror a known-good pattern instead of inventing.

- **Parity target (external):** moka's `Cache` — https://docs.rs/moka/latest/moka/future/struct.Cache.html. The operation set (`get`, `insert`, `invalidate`, `get_with`/`try_get_with`) is the bar; `SimpleCache` matches the *operations*, not the names (cache-kit chose `Result<>` returns + `remove`/`get_or_insert_with` naming). Don't add ops moka lacks; don't omit ops moka has.
- **In-repo pattern to wrap:** `src/expander.rs` — `SimpleCache` holds `inner: CacheExpander<B>` and forwards. It mirrors the existing constructor/backend conventions in `src/backend/inmemory.rs` for `SimpleCache::default()`. It must NOT duplicate backend/serialization/TTL logic (Invariant 5).
- **TTL primitive to reuse:** `TtlPolicy::Fixed(Duration)` in `src/observability.rs` — `with_ttl` wires to it; per-insert TTL is the only genuinely new primitive.
- **README/quick-start reference:** `examples/basic_usage.rs` is the comprehensive full-pattern example; the new `examples/simple_usage.rs` is its ≤30-line sibling that the README mirrors line-for-line.
- **Divergence:** moka returns values directly (panics-on-poison internally); cache-kit returns `Result<>` because the backend can be a network hop (Redis) that genuinely fails. No thundering-herd dedup in v0.9.2 (moka's `get_with` dedupes) — an explicit, documented gap.

---

## Before / After — first-touch DX

**Before (today's `README.md` Quick Start, ~50 lines of Rust):**

```rust
use cache_kit::{
    CacheEntity, CacheFeed, DataRepository, CacheExpander,
    backend::InMemoryBackend, strategy::CacheStrategy,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct User { id: String, name: String }

impl CacheEntity for User {
    type Key = String;
    fn cache_key(&self) -> Self::Key { self.id.clone() }
    fn cache_prefix() -> &'static str { "user" }
}

struct UserFeeder { id: String, user: Option<User> }
impl CacheFeed<User> for UserFeeder {
    fn entity_id(&mut self) -> String { self.id.clone() }
    fn feed(&mut self, entity: Option<User>) { self.user = entity; }
}

struct UserRepository;
impl DataRepository<User> for UserRepository {
    async fn fetch_by_id(&self, id: &String) -> cache_kit::Result<Option<User>> {
        Ok(Some(User { id: id.clone(), name: "Alice".to_string() }))
    }
}

#[tokio::main]
async fn main() -> cache_kit::Result<()> {
    let backend = InMemoryBackend::new();
    let expander = CacheExpander::new(backend);
    let mut feeder = UserFeeder { id: "user_001".to_string(), user: None };
    let repository = UserRepository;
    expander.with(&mut feeder, &repository, CacheStrategy::Refresh).await?;
    Ok(())
}
```

**After (post-M092_003 README Quick Start, ≤ 15 lines of Rust):**

```rust
use cache_kit::SimpleCache;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct User { id: String, name: String }

#[tokio::main]
async fn main() -> cache_kit::Result<()> {
    let cache: SimpleCache<String, User> = SimpleCache::default();
    cache.insert("user_1".into(), User { id: "user_1".into(), name: "Alice".into() }).await?;
    let user: Option<User> = cache.get("user_1".into()).await?;
    println!("{:?}", user.map(|u| u.name));
    Ok(())
}
```

Same library. Same backend. Same serialization envelope. The full pattern still ships, linked from a "Going further: read-through with repositories" section that points to `site/content/v0.9.2/concepts/expander-and-repository.mdx`.

---

## Files Changed (blast radius)

| File | Action | Why |
|------|--------|-----|
| `src/simple.rs` | CREATE | `SimpleCache<K, V, B>` struct, builder, all public methods. Bounded at 350 lines. |
| `src/lib.rs` | EDIT | `pub use simple::SimpleCache;` at the crate root for one-import wiring. |
| `src/expander.rs` | EDIT (small) | Add a `set_with_ttl(key, bytes, ttl)` internal method if not already present, so `SimpleCache::insert_with_ttl` has a primitive to delegate to. No public API change to `CacheExpander`. |
| `src/backend/mod.rs` | EDIT (small, conditional) | If the `Backend` trait lacks per-call TTL (currently uses `TtlPolicy` at the cache level), add an additive default-method `set_with_ttl(...)` that delegates to `set` and ignores TTL. Backends that support per-call TTL (Redis, Memcached) override. The agent confirms whether this is needed before touching. |
| `tests/simple_cache_test.rs` | CREATE | Coverage for every public method, the simplicity invariant doc-test, the README-budget regression. |
| `examples/simple_usage.rs` | CREATE | ≤ 30 lines. README's Quick Start mirrors this file line-for-line. |
| `README.md` | EDIT | Replace `## Quick Start` body with the SimpleCache 15-line version. Add `## Going further` section linking to the repository-pattern docs. Old example moves out of README; the file shrinks. |
| `site/content/v0.9.2/getting-started/quick-start.mdx` | CREATE (or EDIT if M092_002 §7 created it first) | Mirrors the new README Quick Start; expanded with a 30-line walk-through. |
| `site/content/v0.9.2/concepts/simple-vs-expander.mdx` | CREATE | One-page guide: when to use `SimpleCache` vs the full `CacheExpander` pattern. Audience: a reader on the new Quick Start asking "wait, what's the full pattern for?" |
| `site/content/v0.9.2/concepts/expander-and-repository.mdx` | CREATE (or MOVE existing concept page) | The home of the full read-through pattern. Linked from README's "Going further." |
| `scripts/check-readme-quickstart-budget.sh` | CREATE | Counts source lines between `## Quick Start` and the next `##`; fails CI if > 50. |
| `.github/workflows/ci.yml` | EDIT | Wire the README-budget script into CI. |
| `CHANGELOG.md` | EDIT | `## [0.9.2]` section gets a "Add `SimpleCache<K, V>` — moka-parity entry point" bullet alongside M092_001 + M092_002 bullets. |
| `Cargo.toml` | EDIT | Bump `version = "0.9.2"` (idempotent across the three v0.9.2 specs — first one to land owns the bump; the others verify). |
| `VERSION` | EDIT | `0.9.2`. |

---

## Decomposition & alternatives (patch vs refactor)

> Match solution-size to problem-size; surface the call before approval.

- **Chosen shape:** a thin new `SimpleCache` wrapper over the existing `CacheExpander`, plus README/quick-start rewrite. Zero change to the existing pattern's public surface (Invariant 6). The wrapper owns no caching logic — it's an on-ramp, not a second engine.
- **Alternatives considered:**
  1. *Simplify `CacheExpander` itself to need fewer traits.* Rejected — that reshapes the existing differentiator and breaks current consumers; the repository pattern is genuinely useful and stays first-class. Two on-ramps, one engine.
  2. *Make `SimpleCache` a standalone in-memory map (not backed by `CacheExpander`).* Rejected — it would duplicate backend/serialization/TTL and drift from the real engine; a `SimpleCache` user couldn't later point it at Redis. Wrapping keeps one source of truth.
  3. *Add a `SimpleCacheBuilder` now.* Rejected for v0.9.2 — three constructors cover the use cases; a builder is a v1.0 concern (named in Out of Scope) when the option set outgrows three knobs.
- **Patch-vs-refactor verdict:** this is an **additive patch** (new type + docs), explicitly *not* a refactor of the existing surface. The only existing-file change is a possible small internal `set_with_ttl` primitive on `CacheExpander`/`Backend`, gated on confirming it's actually absent first.

---

## Sections (implementation slices)

### §1 — `SimpleCache<K, V, B>` core type + constructors

Deliver: `pub struct SimpleCache<K, V, B = InMemoryBackend>` with `K: Hash + Eq + Send + Sync + Serialize + DeserializeOwned + Clone + Display + 'static`, `V: Send + Sync + Serialize + DeserializeOwned + Clone + 'static`, `B: Backend`. Constructors:

- `SimpleCache::default()` — infallible, returns `SimpleCache<K, V, InMemoryBackend>`. **This is the moka-parity constructor.** No args, no traits, ready to use.
- `SimpleCache::new(backend: B)` — explicit backend.
- `SimpleCache::with_ttl(backend: B, default_ttl: Duration)` — explicit backend + default TTL applied to every insert that doesn't override.

**Implementation default:** the struct holds `inner: CacheExpander<B>` plus `default_ttl: Option<Duration>` and `_phantom: PhantomData<(K, V)>`. The `K: Display` bound is used to format the cache key; the `K: Hash + Eq` bounds future-proof for in-process variants but aren't strictly required today (the agent picks: keep them as forward-compat, or drop them to reduce friction). `V` is serialized via the existing postcard envelope — no envelope changes.

### §2 — Read / write / remove operations

Deliver: the cache-kit-native `Result<>` surface the user picked.

```
impl<K, V, B: Backend> SimpleCache<K, V, B> {
    pub async fn get(&self, key: K) -> Result<Option<V>>;
    pub async fn insert(&self, key: K, value: V) -> Result<()>;
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()>;
    pub async fn remove(&self, key: K) -> Result<()>;
    pub async fn clear(&self) -> Result<()>;
}
```

**Implementation default:** every method delegates to `self.inner` (the wrapped `CacheExpander`). `insert` uses `default_ttl` if set, else falls through to `TtlPolicy::Default`. `insert_with_ttl` always uses the per-call TTL regardless of the default. `clear` is bounded by what the backend supports — InMemory and Redis both can; Memcached's `flush_all` is the equivalent. If a backend can't clear, document it in that backend's module and have `clear()` return `Error::Unsupported` (or whatever the M092_001 code is for that case).

### §3 — `get_or_insert_with` (the moka killer-feature parity)

Deliver: two variants — infallible loader and fallible loader.

```
pub async fn get_or_insert_with<F, Fut>(&self, key: K, loader: F) -> Result<V>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = V> + Send;

pub async fn try_get_or_insert_with<F, Fut, E>(&self, key: K, loader: F) -> Result<V>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = std::result::Result<V, E>> + Send,
    E: Into<Error>;
```

**Implementation default:** check cache; on miss, run loader; on loader success, insert with `default_ttl` and return. `try_*` variant short-circuits the insert on loader error and propagates via `Into<Error>`. **No thundering-herd protection in v0.9.2** — concurrent callers each run the loader independently; deduplication is a v1.0 concern (matches moka's `get_with` semantics, which dedupes; this is an explicit gap and is documented in the doc-comment with a v1.0 link). The agent does NOT silently add deduplication; it's an `Out of Scope` item below.

### §4 — README + Quick Start rewrite

Deliver: `examples/simple_usage.rs` (≤ 30 lines, including imports and entity), and `README.md` Quick Start that mirrors it line-for-line. Old README example moves to `site/content/v0.9.2/concepts/expander-and-repository.mdx` under a clear "When to use this instead of `SimpleCache`" framing.

The README's Quick Start budget is enforced by `scripts/check-readme-quickstart-budget.sh` running in CI — 50 source lines max between `## Quick Start` and the next `##`. Today's count: ~60. Target after this spec: ≤ 30 (the 15 Rust lines plus prose, headings, and the Cargo.toml block).

### §5 — Concept pages: when to use which

Deliver: `site/content/v0.9.2/concepts/simple-vs-expander.mdx` answering one question — "I'm a Rust engineer reading the README; should I use `SimpleCache` or `CacheExpander`?" — with a small decision table. Bounded at 200 lines.

| You want... | Use |
|---|---|
| A `HashMap` with eviction / TTL | `SimpleCache` |
| Read-through caching with a database fallback | `SimpleCache::get_or_insert_with` (light) OR `CacheExpander` + `Repository` (heavy / multi-strategy) |
| Multiple cache strategies (Refresh / Fresh / Invalidate / Bypass) per call site | `CacheExpander` |
| Per-entity-type cache prefixes managed by the framework | `CacheExpander` (`CacheEntity` trait does this) |
| Service-layer pattern with separated concerns (entity, feeder, repository) | `CacheExpander` |
| To migrate from moka with minimum code change | `SimpleCache` |

---

## Interfaces

> Lock the contract. Every existing public name keeps its existing signature. `SimpleCache` is the only new public type.

```
pub struct SimpleCache<K, V, B = InMemoryBackend>
where
    K: Hash + Eq + Send + Sync + Serialize + DeserializeOwned + Clone + Display + 'static,
    V: Send + Sync + Serialize + DeserializeOwned + Clone + 'static,
    B: Backend;

impl<K, V> SimpleCache<K, V, InMemoryBackend> {
    pub fn default() -> Self;
}

impl<K, V, B: Backend> SimpleCache<K, V, B> {
    pub fn new(backend: B) -> Self;
    pub fn with_ttl(backend: B, default_ttl: Duration) -> Self;

    pub async fn get(&self, key: K) -> Result<Option<V>>;
    pub async fn insert(&self, key: K, value: V) -> Result<()>;
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()>;
    pub async fn remove(&self, key: K) -> Result<()>;
    pub async fn clear(&self) -> Result<()>;

    pub async fn get_or_insert_with<F, Fut>(&self, key: K, loader: F) -> Result<V>
    where F: FnOnce() -> Fut, Fut: Future<Output = V> + Send;

    pub async fn try_get_or_insert_with<F, Fut, E>(&self, key: K, loader: F) -> Result<V>
    where F: FnOnce() -> Fut, Fut: Future<Output = std::result::Result<V, E>> + Send, E: Into<Error>;
}
```

**Out of contract for v0.9.2:**
- `SimpleCache::contains_key`, `entry()`, `len()`, `iter()`, `policy()`, eviction-listener callbacks. Each is a small additional spec when picked up; none are required for the moka-parity entry-point goal.
- Thundering-herd deduplication on `get_or_insert_with`. Documented gap; v1.0 work.
- `SimpleCache` builder pattern (`SimpleCacheBuilder::new()...build()`). Three constructors are enough for v0.9.2; a builder is a v1.0 concern when the option set grows.
- Sync (non-async) variant. Cache-kit is async-only (per `README.md` requirements section); SimpleCache inherits that.

---

## Failure Modes

| Mode | Cause | Handling |
|------|-------|----------|
| User picks `SimpleCache` and later needs strategy / repository pattern | Misjudged the use case | `SimpleCache` and `CacheExpander` coexist. Migration path: stop using `SimpleCache`, build a `CacheExpander` directly. The shared backend means cached data persists across the migration. Documented in `simple-vs-expander.mdx`. |
| `K: Display` chosen format collides | Two `K` instances format to the same string (e.g. `i64::MAX as i32`) | `K: Display + Hash + Eq` bounds catch the type-level case at compile time. The textual collision is a user concern; document "use a fresh-from-`Display` key strategy or wrap in a newtype." |
| `get_or_insert_with` thundering herd | Multiple callers miss simultaneously and each runs the loader | Documented in the method's doc-comment as a v0.9.2 limitation with a v1.0 link. Users who need dedup wrap their own `OnceCell` / `tokio::sync::Mutex`. |
| `clear()` on a backend that doesn't support it | Memcached partial support, future backends without flush | Returns `Error::Unsupported` (or M092_001's equivalent code). Documented per-backend. |
| README quick-start drifts from `examples/simple_usage.rs` | Author edits one without the other | `scripts/check-readme-quickstart-budget.sh` runs a diff between the README's Quick Start code block and `examples/simple_usage.rs` and fails CI if they diverge. |
| User expects sync API | Coming from `std::collections::HashMap` mental model | README explicitly states "async-only" in its existing requirements section; SimpleCache inherits. No change needed. |
| Existing `CacheExpander` users mistakenly think `SimpleCache` deprecates their code | Adoption miscommunication | `simple-vs-expander.mdx` opens with "Both surfaces are first-class." CHANGELOG entry says "additive — `CacheExpander` is unchanged and not deprecated." |

---

## Invariants

1. **Zero required user traits** — every public `SimpleCache` method works with a struct that derives only `Serialize + Deserialize + Clone`. Enforced by a doc-test in `src/simple.rs` that defines such a struct and exercises every method.
2. **README quick-start ≤ 50 source lines** — between `## Quick Start` and the next `##`. Enforced by `scripts/check-readme-quickstart-budget.sh` in CI.
3. **`examples/simple_usage.rs` ≤ 30 lines** — enforced by a CI line-count check.
4. **README quick-start ≡ `examples/simple_usage.rs`** — the Rust block in the README must equal the example file (or be a strict prefix). Enforced by `scripts/check-readme-quickstart-budget.sh`.
5. **No duplication of backend / serialization logic** — `SimpleCache` only holds an `inner: CacheExpander<B>` and forwards. Enforced by code review (no direct `Backend` calls from `simple.rs`; all paths go through `inner`).
6. **`CacheExpander` public surface is unchanged** — diff against v0.9.0 shows additions only (the internal `set_with_ttl` if needed). Enforced by `cargo public-api` diff or manual review.
7. **Semver-additive** — the only new public name is `SimpleCache`. No renames, no removals.

---

## Test Specification

> Prose-and-assertions only. The implementing agent writes the actual test code in project style.

| Test | Asserts |
|------|---------|
| `test_simple_cache_default_constructor_works` | `SimpleCache::<String, User>::default()` compiles, returns a usable cache, no traits implemented on `User` beyond derives. |
| `test_simple_cache_get_returns_none_on_miss` | Fresh cache, `cache.get("nope".into()).await` returns `Ok(None)`. |
| `test_simple_cache_insert_then_get_round_trip` | Insert `User { ... }`, get same key returns `Ok(Some(User { ... }))` — value matches. |
| `test_simple_cache_remove_deletes_entry` | Insert, remove, get returns `Ok(None)`. |
| `test_simple_cache_clear_empties_cache` | Insert N entries, clear, every get returns `Ok(None)`. |
| `test_simple_cache_default_ttl_applied` | Construct `with_ttl(backend, 1s)`; insert; sleep 1.5s; get returns `Ok(None)` (entry expired). Wall-clock test — guard against flake with generous bound. |
| `test_simple_cache_per_insert_ttl_overrides_default` | Construct with default 10s; `insert_with_ttl(k, v, 100ms)`; sleep 200ms; get returns `Ok(None)`. |
| `test_simple_cache_get_or_insert_with_runs_loader_on_miss` | Loader counter increments on first call; second call hits cache and counter does not increment. |
| `test_simple_cache_try_get_or_insert_with_propagates_loader_error` | Loader returns `Err(MyError)`; method returns `Err(Error::...)`; cache is not populated. |
| `test_simple_cache_zero_required_traits` | Doc-test in `src/simple.rs`: define `struct Plain { id: String, name: String }` with only `Clone + Serialize + Deserialize`; exercise every `SimpleCache` method. Compiles and runs. |
| `test_simple_cache_works_with_redis_backend` | (Behind `#[cfg(feature = "redis")]`.) `SimpleCache::new(RedisBackend::new(...))` round-trips. |
| `test_readme_quickstart_matches_example` | `scripts/check-readme-quickstart-budget.sh` exits 0; the Rust code block in `README.md` Quick Start equals the body of `examples/simple_usage.rs`. |
| `test_readme_quickstart_under_budget` | `scripts/check-readme-quickstart-budget.sh` reports ≤ 50 lines. |
| `test_simple_usage_example_under_budget` | `wc -l examples/simple_usage.rs` ≤ 30. |
| `test_cache_expander_public_surface_unchanged` | `cargo public-api diff main..HEAD` shows no removals or shape changes on `CacheExpander`. |

**Negative tests:** every Failure Mode has a corresponding test (loader error path, unsupported `clear`, missing entry, expired entry). **Edge cases:** empty `K` value (`SimpleCache::<String, _>` with `cache.insert("".into(), v)` works); very large `V` (delegates to backend; not SimpleCache's concern); UTF-8 multi-byte keys.

---

## Acceptance Criteria

- [ ] `cargo build --release` clean — `cargo build --release 2>&1 | tail -3`
- [ ] `cargo test` passes (existing tests + new SimpleCache tests)
- [ ] `cargo test --features redis` passes (SimpleCache works with Redis backend)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `cargo publish --dry-run` succeeds
- [ ] `cargo doc --no-deps` clean (no broken intra-doc links from `SimpleCache` doc comments)
- [ ] No new file over 350 lines
- [ ] `gitleaks detect` clean
- [ ] `examples/simple_usage.rs` runs to completion — `cargo run --example simple_usage` exits 0
- [ ] `examples/simple_usage.rs` ≤ 30 lines — `wc -l examples/simple_usage.rs`
- [ ] `README.md` Quick Start ≤ 50 lines — `scripts/check-readme-quickstart-budget.sh`
- [ ] README Quick Start code block ≡ `examples/simple_usage.rs` body — same script
- [ ] `cargo public-api diff` shows no removals or shape changes on `CacheExpander`
- [ ] `CHANGELOG.md` `[0.9.2]` includes `SimpleCache` bullet
- [ ] `VERSION` and `Cargo.toml` agree on `0.9.2`
- [ ] CI workflow runs the README-budget script

---

## Eval Commands (Post-Implementation Verification)

```bash
# E1: Default build + test sweep
cargo build --release 2>&1 | tail -3
cargo test 2>&1 | tail -5
cargo test --features redis 2>&1 | tail -5

# E2: Lint, fmt, docs
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -3
cargo fmt --check
cargo doc --no-deps 2>&1 | tail -5

# E3: Example runs
cargo run --example simple_usage 2>&1 | tail -3

# E4: Budget enforcement
wc -l examples/simple_usage.rs   # Expected: <= 30
bash scripts/check-readme-quickstart-budget.sh

# E5: Public API additivity
cargo public-api diff origin/main..HEAD 2>&1 | grep -i 'remov\|chang' || echo "no breaking changes"

# E6: Length gate + secrets + version
git diff --name-only origin/main | grep -E '\.rs$' | xargs wc -l 2>/dev/null | awk '$1 > 350 { print "OVER: " $2 ": " $1 }'
gitleaks detect 2>&1 | tail -3
grep '^version = ' Cargo.toml; cat VERSION
```

---

## Dead Code Sweep

The old README Quick Start example (the 50-line `CacheExpander` walk-through) MOVES to `site/content/v0.9.2/concepts/expander-and-repository.mdx`. Not deleted; relocated. The README itself shrinks. `examples/basic_usage.rs` (243 lines) STAYS — it's still the canonical comprehensive example for the full pattern; it's just no longer the README's pointer-of-first-resort.

---

## Discovery (consult log)

> **Empty at creation.** Append as the work surfaces consults and decisions — the spec's running record where deferrals and skill outcomes are proven.

- **Consults** — {Architecture / Legacy-Design / gate-flag triage: question asked + Indy's decision. Note: the `.github/workflows/ci.yml` edit needs Indy approval — capture it here.}
- **Skill chain outcomes** — {`/write-unit-test`, `/review`, `/review-pr`, `kishore-babysit-prs` results: iteration counts, findings dispositioned.}
- **Deferrals** — every "deferred to follow-up" needs an Indy-acked verbatim quote here, format `> Indy (YYYY-MM-DD HH:MM): "<quote>" — context: <which item, why>`. An agent-unilateral deferral is incomplete scope, not deferral, and blocks CHORE(close).

---

## Skill-Driven Review Chain (mandatory)

| When | Skill | Notes |
|------|-------|------|
| After implementation, before CHORE(close) | `/write-unit-test` | Audit diff coverage vs Test Specification. Particular focus: the simplicity-invariant doc-test (zero required traits), the README-budget regression test, the loader-error propagation in `try_get_or_insert_with`. |
| After tests pass, still before CHORE(close) | `/review` | Adversarial diff review against this spec, `RUST_GUIDELINES.txt`, and `docs/greptile-learnings/RULES.md`. Focus areas: `CacheExpander` surface unchanged (any drift is a violation), no duplicated backend/serialization logic in `simple.rs`, doc-comments on every public method include a working snippet, the "thundering herd" gap is explicitly documented in `get_or_insert_with`'s doc-comment. |
| After `gh pr create` opens the PR | `/review-pr` | Re-review the now-immutable diff. Focus areas: README Quick Start actually reads cleanly (run it through fresh eyes — does it answer "what does this library do?" in 30 seconds?), `simple-vs-expander.mdx` is honest about when to pick which, the CI budget script blocks future bloat. |

---

## Verification Evidence

> Filled in during VERIFY phase.

| Check | Command | Result | Pass? |
|-------|---------|--------|-------|
| Default build | `cargo build --release` | {pending} | |
| Unit tests | `cargo test` | {pending} | |
| Tests with redis | `cargo test --features redis` | {pending} | |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | {pending} | |
| Fmt | `cargo fmt --check` | {pending} | |
| Docs | `cargo doc --no-deps` | {pending} | |
| Example runs | `cargo run --example simple_usage` | {pending} | |
| Example budget | `wc -l examples/simple_usage.rs` | {pending} | |
| README budget | `scripts/check-readme-quickstart-budget.sh` | {pending} | |
| Public API additivity | `cargo public-api diff` | {pending} | |
| 350L gate | `wc -l` on new `.rs` files | {pending} | |
| Gitleaks | `gitleaks detect` | {pending} | |

---

## Out of Scope

- **`CacheExpander` deprecation, removal, or reshape.** The full pattern stays first-class. SimpleCache is additive.
- **Thundering-herd deduplication on `get_or_insert_with`.** Documented v1.0 gap; users who need it wrap their own `OnceCell` or `tokio::sync::Mutex`. Matching moka's dedup semantics requires concurrency primitives that are non-trivial; out of v0.9.2 patch scope.
- **`contains_key`, `len`, `iter`, `entry`, `policy`, eviction listeners.** Each is a small follow-up spec when picked up; none are required for the moka-parity entry-point goal.
- **Builder pattern (`SimpleCacheBuilder`).** Three constructors are enough for v0.9.2; a builder is a v1.0 concern when the option set grows past three knobs.
- **Sync (non-async) variant.** Cache-kit is async-only per `README.md` requirements; SimpleCache inherits.
- **`SimpleCache` integration with M092_001's `ErrorContext`.** Errors returned from SimpleCache will carry M092_001's codes once both ship (the wrapped `CacheExpander` does it for free), but explicit `with_cache_key` annotation on SimpleCache's error returns is a small follow-up that lands when M092_001 is in.
- **`SimpleCache` integration with M092_002's stats / tracing.** SimpleCache will emit M092_002's spans and feed M092_002's stats once both ship (again, free via the wrapped `CacheExpander`). Surfacing a `SimpleCache::stats()` accessor that mirrors `CacheExpander::stats()` is a one-line follow-up; tracked but not in this spec.
- **Cross-version README budget enforcement.** The CI script enforces only the current `README.md`; historical README versions are frozen on past tags.
