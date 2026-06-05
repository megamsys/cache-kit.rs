<!--
SPEC AUTHORING RULES (load-bearing — do not delete):
- No time/effort/hour/day estimates anywhere in this spec.
- No effort columns, complexity ratings, percentage-complete, implementation dates.
- No assigned owners — use git history and handoff notes.
- Priority (P0/P1/P2) is the only sizing signal. Use Dependencies for sequencing.
- If a section below contradicts these rules, the rule wins — delete the section.
- See ~/Projects/dotfiles/docs/TEMPLATE.md for the canonical template.
-->

# M092_002: First-class observability — `tracing` spans, built-in `CacheStats`, `metrics`-crate adapter, and versioned docs

**Prototype:** v0.9.2
**Milestone:** M092
**Workstream:** 002
**Date:** May 16, 2026
**Status:** PENDING
**Priority:** P1 — distributed-cache users (cache-kit's wheelhouse) cannot operate cache-kit in production without writing their own Prometheus / OpenTelemetry glue. The existing `CacheMetrics` trait is a hook with no batteries; this gap is the single biggest reason a Rust engineer evaluating cache-kit alongside moka would say "moka has a stats accessor, cache-kit doesn't." Bundles the docs-site versioning work because the new public surface ships a v0.9.2-specific page that must coexist with future v0.9.x and v1.0 pages.
**Categories:** OBS · API · DOCS
**Batch:** B1 — runs alongside M092_001 (independent code paths, both must land before v0.9.2 release notes).
**Branch:** {feat/m092-observability — added when work begins}
**Depends on:** none (independent of M092_001 — but co-design sensibly: error codes feed `record_error_typed`)
**Provenance:** agent-generated (pre-spec, derived from `plans/comprehensive-code-review.md` §16 #2 "Add Observability Hooks" + §16 #8 "Add Cache Statistics", grounded in actual `src/observability.rs` (189 lines) and `site/lib/content.ts` (275 lines) shape)

> Cache-kit already exposes a `CacheMetrics` trait (`src/observability.rs:85`) and a `NoOpMetrics` default. The trait routes through the `log` crate's `debug!`/`warn!` macros, keys metrics by raw cache key (high-cardinality, breaks Prometheus), and ships no `tracing`, no `metrics`-crate, and no built-in stats collector. This spec turns the existing hook into a usable production surface — additive, feature-gated, no breaking changes to the v0.9.x trait shape — and at the same time adds version-segmented routing to the Next.js docs site so the v0.9.2 observability page can ship without overwriting future pages.

**Canonical architecture:** N/A — no `docs/ARCHITECTURE.md` in this repo. Implementation surface is `src/observability.rs`, callers in `src/expander.rs`, the backend modules in `src/backend/`, plus `site/lib/content.ts` + `site/app/[section]/[slug]/page.tsx` for the docs versioning.

---

## Implementing agent — read these first

1. `src/observability.rs` (189 lines) — current `CacheMetrics` trait, `NoOpMetrics` impl, `TtlPolicy`. **The trait's existing five methods (`record_hit`, `record_miss`, `record_set`, `record_delete`, `record_error`) keep their exact signatures.** New methods land as default-impl additions. The split-out into a `src/observability/` module needs to keep the existing module path's re-exports intact.
2. `src/expander.rs` (908 lines) — every call site that invokes `metrics.record_*`. The new `record_*_typed` calls take `entity_type: &str`; `CacheExpander` already knows `entity_type` per-operation, so wiring is internal — callers are unaffected.
3. `src/backend/redis.rs` (556 lines), `src/backend/memcached.rs`, `src/backend/inmemory.rs` — backend operations are where network-hop time lives. Add `tracing::span!` instrumentation here so users see backend Round-Trip Time (RTT) in their distributed traces.
4. `Cargo.toml` — currently depends on `log = "0.4"`. Add `tracing = "0.1"` (default-on). Add `metrics = { version = "...", optional = true }` behind a new `metrics-adapter` feature.
5. `site/lib/content.ts` (275 lines) and `site/app/[section]/[slug]/page.tsx` (49 lines) — current routing assumes `content/{section}/{slug}.mdx`. To support multiple versions, the loader gains a version dimension, the route gains a `[version]` segment, and a default-version redirect ships at the existing `/{section}/{slug}` paths.
6. `examples/actixsqlx/src/services/user_service.rs` — primary integration consumer. After this spec lands, the example app gets a one-line wiring change (`CacheExpander::new(...).with_tracing()`) plus a `tracing-subscriber` setup line in `main.rs`. Confirm the example still runs.
7. `plans/comprehensive-code-review.md` §16 #2 + §16 #8 (intent only — do NOT copy code) — the recommendations the spec implements. Read **High Priority #2** and **Low Priority #8**; ignore the prose suggestions about OpenTelemetry — that's a separate spec, this one ships `tracing` (which OTel adapters consume) but does not own the OTel adapter wiring.

---

## PR Intent & comprehension handshake

> The bridge from spec to the merged PR. Makes the agent confirm it understood intent *before* writing code.

- **PR title (eventual):** `feat(obs): tracing spans, built-in stats, metrics adapter + versioned docs`
- **Intent (one sentence):** A consumer gets `tracing` spans, an `expander.stats()` accessor, and feature-gated Prometheus counters from one-line wiring — without writing a custom `CacheMetrics` impl or breaking any v0.9.0 impl — while the docs site serves per-version pages.
- **Handshake (agent fills at PLAN, before EXECUTE):** the implementing agent restates the intent in its own words and lists `ASSUMPTIONS I'M MAKING: …`. A mismatch between that restatement and the Intent above → STOP and reconcile before any edit.

---

## Applicable Rules

- **Project rule source:** `RUST_GUIDELINES.txt` at repo root — read sections covering trait additivity, feature-flag hygiene, zero-cost abstractions.
- **`docs/greptile-learnings/RULES.md`** — universal repo discipline; applies to the diff.
- **Semver discipline** — this is a v0.9.x change; the public `CacheMetrics` trait is part of the consumed Application Programming Interface (API). Changes must be additive (new methods with default impls, new types, new builder methods) for v0.9.2. Removing or re-typing existing methods is a v1.0 concern and out of scope here.
- **Cardinality discipline** — Prometheus / metrics-crate label cardinality blowup is a production incident waiting to happen. Cache key MUST NOT be used as a metric label; entity type MUST be the only high-cardinality axis, and entity types are bounded (one per `Entity` impl in the consumer codebase).
- **Feature-flag hygiene** — `cargo build --no-default-features` and `cargo build --all-features` both stay green. The `metrics-adapter` feature gates only the `metrics`-crate dependency, never trait shape.
- **Docs versioning rule (new, set by §7)** — every public-API-affecting spec from v0.9.2 forward authors its docs page under `site/content/v{X.Y.Z}/{section}/{slug}.mdx`. The `latest` alias points to the highest-version directory at build time.

---

## Applicable Gates

> Which Action-Triggered Guards this PR WILL trip, and how each stays clean. Rules ≠ Gates: rules are knowledge to read; gates fire on edits.

| Gate | Fires? | Satisfaction strategy |
|------|--------|-----------------------|
| File & Function Length (≤350/≤50/≤70) | yes — `expander.rs` is 908 lines, `observability.rs` 189 | Module split into `src/observability/{metrics,ttl,stats,tracing_adapter,metrics_crate}.rs`; each new file < 350. `expander.rs` only gains instrument attrs + builder methods, not bulk. |
| PUB / Struct-Shape | yes — `StatsCollector`, `CacheStatsSnapshot`, `TracingMetrics`, `MetricsCrateAdapter`, 5 new trait defaults, 4 builder methods | All additive; existing `CacheMetrics` method signatures frozen. Doc-comment every new pub item. |
| UFS (repeated/semantic literals) | yes — metric/span names (`cache_hits_total`, `cache.hit`, etc.) | Each stable name is a named `const` shared by adapter + tests verbatim (Invariant 6 depends on stability). |
| LOGGING | yes — moves from `log` macros toward `tracing` events | Stable event names + fields per `LOGGING_STANDARD`; no raw cache key in events by default (Invariant 4). |
| Feature-flag hygiene (not a gate but CI-enforced) | yes — `metrics-adapter` feature | `#[cfg(feature = "metrics-adapter")]` on every `metrics`-crate reference; `--no-default-features` + `--all-features` both green. |
| UI Substitution / DESIGN TOKEN | yes — new `VersionSelector.tsx` in the docs site | Use the site's existing component + token conventions in `site/components/`; no raw arbitrary values. |
| SCHEMA / ZIG | no | No `*.sql` or `*.zig` touched. |

---

## Overview

**Goal (testable):** A consumer can do `cargo add cache-kit --features tracing,metrics-adapter` and, with a one-line wiring change (`CacheExpander::new(...).with_tracing().with_metrics_crate()`), get (a) `cache.get` / `cache.set` / `cache.backend.<name>.<op>` spans in their distributed-trace pipeline, (b) `cache_hits_total{entity_type="user"}` / `cache_op_duration_seconds` counters in their Prometheus scrape, and (c) a `expander.stats()` accessor returning a `CacheStatsSnapshot` (hits, misses, errors, bytes_in, bytes_out, op_count) — without writing any custom `CacheMetrics` impl, without changing existing call sites, and without breaking any v0.9.0 consumer's existing `CacheMetrics` impl. Concurrently, the docs site at `cachekit.org` serves both `/v0.9.2/integration/observability` and `/latest/integration/observability`, and a version selector in the sidebar lets readers switch.

**Problem:** Today, cache-kit's observability story stops at "implement this trait yourself." Three observable consequences:
1. **Operators cannot answer "is my cache helping?"** — there's no built-in hit/miss accessor; every consumer must wire their own counter, and most don't, so caches ship un-instrumented.
2. **Cache operations are invisible in distributed traces.** Modern Rust services pipe everything through `tracing` → OpenTelemetry → Datadog/Honeycomb. Cache-kit emits via the `log` crate, so its calls appear as detached log lines instead of nested spans inside the request waterfall. Latency attribution is impossible.
3. **The trait's `&str` key parameter invites cardinality blowup.** A naive consumer wires `record_hit(key, ...)` straight into Prometheus and discovers their cardinality went from 10 to 10M overnight. The trait gives no safe default; the entity-type axis (10s of values) doesn't exist on the trait surface at all.

Plus a fourth, docs-side: **the docs site has no version dimension.** Today `/integration/backends` always serves the head of `main`. Once the v0.9.2 surface ships, users on v0.9.0 hit pages describing methods they don't have, and any future v1.0 page that breaks the trait shape forces a coin-flip choice between accuracy-for-old-users and accuracy-for-new-users.

**Solution summary:** Three additive code layers + one docs-site change.
- **Layer 1 — `tracing` instrumentation.** Internal `tracing` macros in `expander.rs` and the backends. Enabled by default once `tracing` is a dep; users opt out by not configuring a `Subscriber`. No public API change required for spans to start emitting.
- **Layer 2 — Built-in `StatsCollector`.** A `CacheMetrics` impl backed by atomic counters. `CacheExpander::stats() -> CacheStatsSnapshot` returns a copy. Zero allocation in hot path. Always available regardless of features.
- **Layer 3 — `metrics`-crate adapter (`metrics-adapter` feature).** A `MetricsCrateAdapter` impl that emits `cache_hits_total`, `cache_misses_total`, `cache_errors_total`, `cache_op_duration_seconds`, `cache_bytes_total` against the global `metrics` recorder. Users wire any recorder (Prometheus, StatsD, OTLP) — cache-kit doesn't pick.
- **Docs change — version-segmented routing.** Content tree gains a `v{X.Y.Z}` top-level dir. Routes gain a `[version]` segment. Existing `/{section}/{slug}` paths redirect to `/latest/{section}/{slug}`. Sidebar gets a version selector.

The `CacheMetrics` trait grows five new default-method `_typed` variants that take `entity_type: &str`. Existing impls keep working (default delegates to the un-typed method); new internal callers always pass entity type, so the built-in collectors get bucketed metrics for free.

---

## Prior-Art / Reference Implementations

> Mirror a known-good pattern instead of inventing.

- **In-repo pattern to mirror:** `src/observability.rs` — the existing `CacheMetrics` trait + `NoOpMetrics` is the shape the new `_typed` defaults and adapter impls extend. The module-split + re-export-shim pattern keeps `cache_kit::observability::*` import paths valid.
- **Stats accessor precedent:** moka's `Cache::entry_count()` / its stats surface — the competitive bar this spec matches with `expander.stats() -> CacheStatsSnapshot`. `metrics`-crate counter naming follows the Prometheus `*_total` convention.
- **Tracing precedent:** `tower-http`'s `TraceLayer` (stable span/event names, `skip` on large args, hashed/redacted sensitive fields) is the canonical Rust instrumentation pattern; `#[tracing::instrument]` usage mirrors it.
- **Docs versioning precedent:** the existing `site/lib/content.ts` `fs.readdirSync` content loader — §7 extends that filesystem-as-source-of-truth pattern with a version dimension rather than introducing per-version git branches or a parallel build pipeline.
- **Divergence:** cache-kit emits `tracing` + `metrics`-crate generically and ships *no* first-party OpenTelemetry/StatsD adapter (those are downstream exporters), keeping the crate framework-agnostic.

---

## Before / After — DX impact on the "should I pick cache-kit?" decision

| Decision question a Rust engineer asks | Today (v0.9.0) | After this spec (v0.9.2) |
|----------------------------------------|----------------|--------------------------|
| "Can I see cache hits/misses without wiring my own counter?" | ❌ Implement the `CacheMetrics` trait yourself. | ✅ `expander.stats().hit_rate()` — one line. |
| "Will cache ops show up in my Datadog/Honeycomb trace waterfall?" | ❌ No `tracing` integration; operations are invisible inside request spans. | ✅ `cache.get`/`cache.set` spans nest under the caller's span; backend RTT is its own child span. |
| "Can I scrape Prometheus metrics without writing a custom adapter?" | ❌ Trait + `Box::new(MyPrometheusImpl)`; figure out cardinality yourself. | ✅ `cargo add cache-kit --features metrics-adapter`, call `with_metrics_crate()`, named counters appear under any `metrics` recorder. |
| "Will a naive Prometheus wiring blow up my cardinality?" | ⚠️ Yes — the trait's `&str` key arg invites it. | ✅ Adapter emits only `entity_type` as a label by default; raw key requires explicit opt-in. |
| "Does this respect my PII boundaries?" | ⚠️ Trait gets the raw key; nothing stops a buggy impl from logging emails. | ✅ `TracingMetrics` defaults to `key.hash` (xxhash64-hex); raw keys require `with_raw_keys(true)` opt-in. |
| "If I'm reading docs for v0.9.0, do I get v0.9.0 docs?" | ❌ Site always serves head of `main`. | ✅ `/v0.9.0/...`, `/v0.9.2/...`, `/latest/...` all coexist. |
| "Can I keep my existing v0.9.0 `CacheMetrics` impl?" | n/a | ✅ Trait change is additive; defaults delegate to the un-typed methods. |

**The honest sentence this unlocks:** *"Use cache-kit when you need a distributed cache that integrates with your existing `tracing` + Prometheus pipeline; use moka when your working set fits in memory and a single process."* Today, the second clause is true and the first isn't — both halves of the recommendation matter, and right now we can only earn the second.

---

## Migration approach for existing v0.9.x consumers

> Goal: zero forced code changes. Any v0.9.0 / v0.9.1 consumer can `cargo update -p cache-kit` to v0.9.2, rebuild, and ship — no diff required. Adopting the new surface is opt-in, in any order, one feature at a time.

**Migration tiers (each tier is independent — adopt zero, one, or all):**

| Tier | What you change | What you get |
|------|-----------------|--------------|
| **Tier 0 — Just upgrade** | `cargo update -p cache-kit` (or bump `Cargo.toml` to `0.9.2`). | Internal `tracing` spans start emitting if you already have a `Subscriber` configured. Existing `CacheMetrics` impls keep compiling. Zero source change. |
| **Tier 1 — Built-in stats** | Add `.with_stats()` to your `CacheExpander` builder; call `.stats()` where you want a snapshot. | Hit/miss/error/byte counters via `CacheStatsSnapshot::hit_rate()` etc. ~3 lines of code. |
| **Tier 2 — `tracing` adapter** | Add `.with_tracing()` to the builder. Make sure you have a `tracing-subscriber` configured. | Stable-named events (`cache.hit`, `cache.miss`, …) with `entity_type` and `key.hash` fields. ~1 line of code. |
| **Tier 3 — Prometheus / `metrics` crate** | Add `metrics-adapter` to your features in `Cargo.toml`; add `.with_metrics_crate()` to the builder; configure a recorder (e.g. `metrics-exporter-prometheus`). | Named counters/histograms scrapeable from your existing Prometheus exporter. ~5 lines of code + recorder setup. |
| **Tier 4 — Custom `CacheMetrics` impl using new `_typed` methods** | Override `record_hit_typed` (etc.) instead of `record_hit`. | Per-entity-type bucketing in your custom backend (e.g. emit different counters per entity type). |

**Backward-compat guarantees (enforced by tests in `tests/regression_v090_compat.rs`):**

1. **`use cache_kit::observability::{CacheMetrics, NoOpMetrics, TtlPolicy};`** — still works after the module split. The shim re-exports.
2. **A v0.9.0 impl `impl CacheMetrics for MyMetrics { fn record_hit(...) {...} ... }`** — still compiles. The new `_typed` methods carry default impls that delegate.
3. **`CacheExpander::with_metrics(Box::new(my_metrics))`** — still works. New `with_tracing` / `with_stats` / `with_metrics_crate` are additive builder methods, not replacements.
4. **No new required trait methods.** Every addition is `fn ... { default_impl(); }`.

**What v0.9.x consumers should NOT do:**

- Don't override `record_hit` AND `record_hit_typed` in the same impl — the un-typed default is no longer called from inside cache-kit (internal callers use `_typed`); your override of the un-typed variant becomes dead code unless something else still calls it. Pick one.
- Don't pass raw user-PII strings as `entity_type` — that defeats the cardinality contract. `entity_type` should be the symbolic name (`"user"`, `"session"`), not an instance value.

**Communicated via:** a "Migrating to v0.9.2" page at `site/content/v0.9.2/guides/migrating-from-v0.9.x.mdx` and a `## Migration` subsection in the v0.9.2 CHANGELOG entry.

---

## Files Changed (blast radius)

| File | Action | Why |
|------|--------|-----|
| `src/observability.rs` | EDIT | Convert into a module re-export shim. Keeps `pub use` of `CacheMetrics`, `NoOpMetrics`, `TtlPolicy` so v0.9.0 consumers' import paths stay valid. |
| `src/observability/mod.rs` | CREATE | New module root. Re-exports trait + adapters + stats. |
| `src/observability/metrics.rs` | CREATE | `CacheMetrics` trait (moved verbatim, plus new `_typed` default-method variants) + `NoOpMetrics`. |
| `src/observability/ttl.rs` | CREATE | `TtlPolicy` (moved verbatim). |
| `src/observability/stats.rs` | CREATE | `StatsCollector` (atomic-counter `CacheMetrics` impl) + `CacheStatsSnapshot` POD. |
| `src/observability/tracing_adapter.rs` | CREATE | `TracingMetrics` impl emitting stable-named `tracing::event!` records. |
| `src/observability/metrics_crate.rs` | CREATE | `MetricsCrateAdapter` behind `#[cfg(feature = "metrics-adapter")]`. |
| `src/expander.rs` | EDIT | Wrap public ops with `#[tracing::instrument]` (skip large args). Replace `metrics.record_*(key, ...)` with `metrics.record_*_typed(entity_type, key, ...)`. Add `with_tracing()`, `with_stats()`, `with_metrics_crate()`, `stats()`. |
| `src/backend/{redis,memcached,inmemory}.rs` | EDIT | `tracing::span!` around backend calls; field tags `backend`, `op`, `key.hash`. Key hashed (not raw) for non-PII default. |
| `src/lib.rs` | EDIT | Re-export `CacheStatsSnapshot`, `TracingMetrics`, `StatsCollector` at crate root. |
| `Cargo.toml` | EDIT | Add `tracing = "0.1"`. Add `metrics = { version = "...", optional = true }`. Define `[features] metrics-adapter = ["dep:metrics"]`. Bump `version = "0.9.2"`. |
| `tests/observability_test.rs` | CREATE | Coverage for stats counters, tracing field shape, metrics adapter, no-default-features build. |
| `tests/regression_v090_compat.rs` | CREATE | Pin v0.9.0 trait shape + import paths via fixture impl. |
| `examples/observability/` | CREATE | Standalone example: `tracing-subscriber` + Prometheus exporter + `CacheExpander`. |
| `site/lib/content.ts` | EDIT | Add version dimension to `loadPage` / `listAllPages` / `getSidebar`. Default version = highest `v*` dir present. |
| `site/app/[version]/[section]/[slug]/page.tsx` | CREATE | New dynamic route with version segment. |
| `site/app/[section]/[slug]/page.tsx` | EDIT | Becomes a redirect-to-`/latest/{section}/{slug}` shim for backwards-compat URLs. |
| `site/lib/versions.ts` | CREATE | `listVersions()`, `latestVersion()`, `defaultVersionRedirect()` — single source of truth for version discovery. |
| `site/content/v0.9.2/integration/observability.mdx` | CREATE | The user-facing observability page (this spec's docs deliverable). |
| `site/content/v0.9.2/guides/migrating-from-v0.9.x.mdx` | CREATE | The migration page documented above. |
| `site/content/{getting-started,concepts,integration,guides}/*.mdx` | MOVE | Existing pages become `site/content/v0.9.2/{section}/*.mdx` — there is no v0.9.0 content tree (we don't retroactively author historical docs); v0.9.2 is the first versioned snapshot. |
| `site/components/VersionSelector.tsx` | CREATE | Sidebar dropdown; reads from `listVersions()`. |
| `CHANGELOG.md` | EDIT | `## [0.9.2]` section: observability bullet + docs-versioning bullet alongside M092_001's bullet. Includes the Migration subsection. |
| `VERSION` | EDIT | `0.9.2`. |
| `plans/comprehensive-code-review.md` §16 | EDIT (annotation only) | Mark #2 + #8 as SPEC'D linking to this spec. |

---

## Decomposition & alternatives (patch vs refactor)

> Match solution-size to problem-size; surface the call before approval.

- **Chosen shape:** three independent additive code layers (tracing / built-in stats / feature-gated `metrics` adapter) over the existing trait, plus a self-contained docs-versioning slice (§7). Each layer is adopt-zero-or-more; nothing is mutually required.
- **Alternatives considered:**
  1. *Re-type the `CacheMetrics` trait to carry `entity_type` on the existing methods (no `_typed` siblings).* Rejected — it breaks every v0.9.0 impl, forcing a major version. The `_typed` default-delegation pattern gets the same internal benefit additively.
  2. *Ship a first-party Prometheus/OTel adapter instead of the generic `metrics` crate.* Rejected — couples cache-kit to one exporter; the `metrics`-crate facade lets consumers pick (Prometheus/StatsD/OTLP) with no cache-kit dependency churn.
  3. *Docs versioning via git branches per release.* Rejected — adds a per-version build pipeline; the filesystem `v{X.Y.Z}/` tree matches the existing loader and ships docs in the same PR as the code.
- **Patch-vs-refactor verdict:** this is a **patch with a contained internal refactor** (the `observability.rs` → `observability/` module split). The split is bounded by the length gate, not opportunistic. Bundling the docs-versioning slice here is deliberate: the new surface needs a versioned page to live on, and splitting it into its own spec would ship a page with nowhere to route. OTel adapter + cross-version search are named follow-ups in **Out of Scope**.

---

## Sections (implementation slices)

### §1 — Module split + trait extension (additive)

Deliver: move `CacheMetrics`, `NoOpMetrics`, `TtlPolicy` into `src/observability/{metrics,ttl}.rs`. `src/observability.rs` becomes a one-line re-export shim. Add five new methods to `CacheMetrics`:

```
fn record_hit_typed(&self, entity_type: &str, key: &str, duration: Duration) { self.record_hit(key, duration); }
fn record_miss_typed(&self, entity_type: &str, key: &str, duration: Duration) { self.record_miss(key, duration); }
fn record_set_typed(&self, entity_type: &str, key: &str, duration: Duration) { self.record_set(key, duration); }
fn record_delete_typed(&self, entity_type: &str, key: &str, duration: Duration) { self.record_delete(key, duration); }
fn record_error_typed(&self, entity_type: &str, key: &str, error: &str) { self.record_error(key, error); }
```

Default impls delegate to the existing un-typed methods so v0.9.0 user impls keep functioning. Internal call sites in `expander.rs` always invoke the `_typed` variants.

### §2 — `StatsCollector` + `CacheStatsSnapshot`

Deliver: `pub struct StatsCollector` with `AtomicU64` counters (hits, misses, errors, bytes_in, bytes_out, op_count). Implements `CacheMetrics` using `Ordering::Relaxed`. Snapshot POD with a `hit_rate()` helper. `CacheExpander::stats()` returns the snapshot (zero snapshot if no `StatsCollector` wired — no panic).

`fetch_add` is `saturating_add`; document u64 overflow is a non-concern at realistic op rates. No per-entity-type sub-counters in the snapshot itself (flat for v0.9.2; bucketed sub-counters are a v1.0 concern; users who need them wire the `metrics`-crate adapter).

### §3 — `TracingMetrics` adapter + `#[tracing::instrument]` on ops

Deliver: `TracingMetrics` emitting stable event names (`cache.hit`, `cache.miss`, `cache.set`, `cache.delete`, `cache.error`) with stable fields (`entity_type`, `key.hash`, `duration_us`, `error`). Wrap public ops in `expander.rs` with `#[tracing::instrument(skip(self, ...), fields(entity_type, key.hash))]`. Backend modules wrap network calls with `tracing::span!` so backend RTT lands as a child span.

Raw cache keys are NOT emitted by default — they often contain PII. `key.hash` is xxhash64-hex. Opt in via `TracingMetrics::new().with_raw_keys(true)` with a doc-comment warning.

### §4 — `metrics`-crate adapter (feature-gated)

Deliver: `#[cfg(feature = "metrics-adapter")] pub struct MetricsCrateAdapter` emitting `cache_hits_total{entity_type=...}`, `cache_misses_total`, `cache_errors_total`, `cache_op_duration_seconds{entity_type, op}`, `cache_bytes_total{entity_type, direction}`. `CacheExpander::with_metrics_crate()` is also gated. Feature-off = dependency absent — `cargo build --no-default-features` clean.

### §5 — `examples/observability/` end-to-end wiring

Deliver: a standalone Cargo example wiring `tracing-subscriber::fmt` + `metrics_exporter_prometheus::PrometheusBuilder` + `CacheExpander::new(InMemoryBackend::new()).with_tracing().with_metrics_crate()`. README explains `cargo run --example observability --features metrics-adapter` and `curl localhost:9000/metrics`.

### §6 — User-facing observability page

Deliver: `site/content/v0.9.2/integration/observability.mdx` covering Three Layers, Tracing setup, Stats accessor, Prometheus / `metrics`-crate, Cardinality contract. Every snippet must compile; agent extracts snippets into the example before pasting.

### §7 — Multi-version docs support (Next.js)

Deliver: a version-segmented content tree and routing layer.

**Content tree (filesystem source of truth):**

```
site/content/
  v0.9.2/
    getting-started/
    concepts/
    integration/
      observability.mdx        ← shipped by this spec
      backends.mdx             ← moved from site/content/integration/
      ...
    guides/
      migrating-from-v0.9.x.mdx ← shipped by this spec
  (future: v0.9.3/, v1.0.0/, ...)
```

**Routing:**

- `site/app/[version]/[section]/[slug]/page.tsx` — primary route. `[version]` matches `v\d+\.\d+\.\d+` OR the literal `latest`.
- `site/app/[section]/[slug]/page.tsx` — kept as a `redirect()` to `/latest/{section}/{slug}` so old inbound links / search-engine crawls don't 404 during the cutover.

**Loader (`site/lib/content.ts`):**

- `loadPage(version: string, section: string, slug: string)` — version added as first arg.
- `listVersions(): string[]` — `fs.readdirSync(content)` filtered to `^v\d+\.\d+\.\d+$`, sorted by semver descending.
- `latestVersion(): string` — `listVersions()[0]`.
- `latest` is a build-time alias resolved to the highest `v*` directory at the loader boundary (no `content/latest/` directory on disk — the alias prevents drift).
- `getSidebar(version)` — same shape as today, scoped to one version.

**Sidebar (`site/components/VersionSelector.tsx`):**

A small dropdown above the section list. Selecting a version navigates to `/{version}/{currentSection}/{currentSlug}` if that page exists in the chosen version, else falls back to `/{version}/getting-started/install`.

**Build-time correctness (`site/scripts/audit-versions.mjs`, run in `npm run build`):**

- Every version dir has a `getting-started/install.mdx` (the version-selector fallback target).
- `listVersions()` is non-empty.
- No file under `site/content/v*/` exceeds 350 lines.
- The `latest` alias resolves to a real directory.

**Why this shape (instead of git branches per version, or a `versions/` parallel tree):**

- **Filesystem-as-source-of-truth** matches the existing `lib/content.ts` design (already does `fs.readdirSync`); we extend that pattern, not replace it.
- **One repo, one build** — Netlify deploys the whole site every push; no per-version build pipeline. Cheap.
- **Authoring locality** — the v0.9.2 spec ships its v0.9.2 docs in the same Pull Request (PR). No branch-shuffle ceremony.
- **Future v1.0** can break the trait shape without rewriting v0.9.x docs — the v0.9.x tree freezes, the v1.0.0 tree is a fresh authoring surface.

**Out of §7 scope (deferred):** search across versions (each version is its own searchable scope for v0.9.2; cross-version search is a v0.9.3 concern), automatic version banners ("you're reading v0.9.2 — view latest"), deep-link migration ("this page in latest is at a different slug"). Each is a small additional spec.

---

## Interfaces

> Lock the contract. Public-API additions only — every existing public name keeps its existing signature.

```
// EXISTING (unchanged shape):
pub trait CacheMetrics: Send + Sync {
    fn record_hit(&self, key: &str, duration: Duration) { ... }
    fn record_miss(&self, key: &str, duration: Duration) { ... }
    fn record_set(&self, key: &str, duration: Duration) { ... }
    fn record_delete(&self, key: &str, duration: Duration) { ... }
    fn record_error(&self, key: &str, error: &str) { ... }

    // NEW additive defaults:
    fn record_hit_typed(&self, entity_type: &str, key: &str, duration: Duration)   { self.record_hit(key, duration); }
    fn record_miss_typed(&self, entity_type: &str, key: &str, duration: Duration)  { self.record_miss(key, duration); }
    fn record_set_typed(&self, entity_type: &str, key: &str, duration: Duration)   { self.record_set(key, duration); }
    fn record_delete_typed(&self, entity_type: &str, key: &str, duration: Duration){ self.record_delete(key, duration); }
    fn record_error_typed(&self, entity_type: &str, key: &str, error: &str)        { self.record_error(key, error); }
}

pub struct StatsCollector { /* atomic counters */ }
impl StatsCollector { pub fn new() -> Self; pub fn snapshot(&self) -> CacheStatsSnapshot; }
impl CacheMetrics for StatsCollector { /* atomic increments */ }

pub struct CacheStatsSnapshot {
    pub hits: u64, pub misses: u64, pub errors: u64,
    pub bytes_in: u64, pub bytes_out: u64, pub op_count: u64,
}
impl CacheStatsSnapshot { pub fn hit_rate(&self) -> f64; }

pub struct TracingMetrics { /* config */ }
impl TracingMetrics { pub fn new() -> Self; pub fn with_raw_keys(self, on: bool) -> Self; }
impl CacheMetrics for TracingMetrics { /* tracing::event! */ }

#[cfg(feature = "metrics-adapter")]
pub struct MetricsCrateAdapter;
#[cfg(feature = "metrics-adapter")]
impl CacheMetrics for MetricsCrateAdapter { /* metrics:: macros */ }

impl<B: Backend> CacheExpander<B> {
    pub fn with_tracing(self) -> Self;
    pub fn with_stats(self) -> Self;
    #[cfg(feature = "metrics-adapter")]
    pub fn with_metrics_crate(self) -> Self;
    pub fn stats(&self) -> CacheStatsSnapshot;
}
```

**Docs-site interfaces:**

```
Routes:
  /{version}/{section}/{slug}    — primary (version ∈ {v0.9.2, ..., latest})
  /{section}/{slug}              — 308 redirect → /latest/{section}/{slug}

Loader (site/lib/content.ts):
  loadPage(version, section, slug): DocPage
  listAllPages(version): DocPage[]
  listVersions(): string[]              // semver desc
  latestVersion(): string
  getSidebar(version): SidebarSection[]
```

**Out of contract:** OpenTelemetry adapter (use `tracing-opentelemetry` downstream). StatsD adapter (route via `metrics`-crate exporter). Per-entity-type sub-counters in `CacheStatsSnapshot`. Histograms in the built-in collector. Cross-version docs search.

---

## Failure Modes

| Mode | Cause | Handling |
|------|-------|----------|
| v0.9.0 user impl no longer compiles | Trait change accidentally non-additive | Default impls on every new `_typed` method delegate to the existing un-typed method. Pinned by `tests/regression_v090_compat.rs`. |
| Prometheus cardinality blowup | User wires raw cache key as a label | `MetricsCrateAdapter` emits only `entity_type` as a label; documented "Cardinality contract" in observability page. |
| PII in distributed traces | Raw keys leak via spans | `TracingMetrics` defaults to `key.hash`; `with_raw_keys(true)` is opt-in with a doc warning. |
| Counter overflow | u64 wraps | `saturating_add`; documented; non-concern at realistic rates. |
| `cargo build --no-default-features` breaks | `metrics` dep accidentally non-optional | `#[cfg(feature = "metrics-adapter")]` on every reference; CI matrix step. |
| `tracing` overhead with no `Subscriber` | Macros expensive when nobody listens | `tracing` macros are near-zero-cost when no subscriber is registered (one atomic load); documented constraint. |
| Double-counting from chained adapters | Builders compose naively | Internal `MultiMetrics` fans out exactly once per adapter; user wiring the same adapter twice is a user bug. |
| Old inbound URL `/integration/backends` 404s after content move | Routes restructured without redirect | Backwards-compat redirect from `/{section}/{slug}` → `/latest/{section}/{slug}` lives in `site/app/[section]/[slug]/page.tsx`. |
| Version-selector fallback dead-ends | Target slug doesn't exist in chosen version | Selector falls back to `/{version}/getting-started/install`; `audit-versions.mjs` enforces every version dir has that page. |
| `latest` alias points to nothing | Empty `site/content/` after refactor | Build-time audit fails; CI blocks merge. |

---

## Invariants

1. **Trait additivity** — every existing v0.9.0 `CacheMetrics` impl keeps compiling unchanged.
2. **No-default-features build clean** — `metrics` crate is absent from `cargo tree --no-default-features`.
3. **Cache key never appears as a metric label.**
4. **Cache key never appears as a trace field by default** — `TracingMetrics::new().raw_keys` is `false`.
5. **Stats counters are wait-free** — only atomic ops, no locks, no allocation in the hot path.
6. **Span event names are stable** — `cache.hit/miss/set/delete/error`, span names `cache.get/set` and `cache.backend.<name>.<op>`. Renaming breaks Grafana dashboards.
7. **Existing v0.9.0 import paths remain valid** — `cache_kit::observability::CacheMetrics` resolves after the module split.
8. **Docs versioning is non-destructive** — every URL that worked before this spec keeps working (via redirect for unversioned URLs).
9. **`latest` always resolves** — build fails if `site/content/v*/` is empty or the alias target is missing.

---

## Test Specification

> Prose-and-assertions only. The implementing agent writes the actual test code in project style.

| Test | Asserts |
|------|---------|
| `test_v090_cachemetrics_impl_still_compiles` | A struct implementing only the original five methods is acceptable as `Box<dyn CacheMetrics>`. Compile-time regression. |
| `test_v090_import_paths_resolve` | `use cache_kit::observability::{CacheMetrics, NoOpMetrics, TtlPolicy};` compiles after the module split. |
| `test_stats_collector_counts` | After 7 hits + 3 misses + 2 errors, snapshot reports correct counts; `hit_rate() == 0.7`. |
| `test_stats_collector_zero_when_unwired` | `CacheExpander::new(...).stats()` returns the zero snapshot (no panic). |
| `test_tracing_emits_stable_fields` | Events captured via `tracing-subscriber` test layer have name `cache.hit`, fields `entity_type`, `key.hash`, `duration_us` — no `key`. |
| `test_tracing_with_raw_keys_emits_key` | `TracingMetrics::new().with_raw_keys(true)` adds a `key` field. |
| `test_metrics_adapter_emits_named_counters` | (Behind `#[cfg(feature = "metrics-adapter")]`.) Debug recorder; 5 hits on `"user"`; assert `cache_hits_total{entity_type="user"} == 5`. |
| `test_metrics_adapter_no_key_label` | No emitted metric carries a `key` label. |
| `test_no_default_features_build` | CI: `cargo build --no-default-features` clean; `cargo tree --no-default-features \| grep -c '^metrics '` is 0. |
| `test_all_features_build` | CI: `cargo build --all-features` clean. |
| `test_typed_methods_default_to_untyped` | Override only `record_hit`; call `record_hit_typed`; assert override fired (default delegation works). |
| `test_expander_emits_op_span` | Drive `expander.get(...)`; assert `cache.get` span exists with `entity_type` field, `cache.backend.<name>.get` nested inside. |
| `test_multi_metrics_no_double_counting` | Install `StatsCollector` + `TracingMetrics`; one hit; stats reports 1, exactly one event emitted. |
| `test_site_loadPage_with_version` | `loadPage("v0.9.2", "integration", "observability")` returns the expected page. |
| `test_site_listVersions_sorted_desc` | With `v0.9.2/` + a stub `v0.9.3/` dir, `listVersions()` returns `["v0.9.3","v0.9.2"]`. |
| `test_site_latest_resolves` | `latestVersion()` returns highest semver dir; `loadPage("latest", ...)` resolves to the same content. |
| `test_site_unversioned_redirects` | Visit `/integration/observability` → 308 redirect to `/latest/integration/observability`. |
| `test_site_audit_passes` | `node site/scripts/audit-versions.mjs` exits 0 on the new tree. |

**Negative tests** match Failure Modes 1:1 (regression v0.9.0, no-default-features, no `key` label, no `key` trace field, redirect for unversioned URLs).
**Edge cases:** empty `entity_type` string (still emits, no panic), UTF-8 multi-byte `entity_type`, very long `entity_type` (>1KB — passed through; cardinality control is the user's job).

---

## Acceptance Criteria

- [ ] `cargo build --release` clean — `cargo build --release 2>&1 | tail -3`
- [ ] `cargo build --no-default-features` clean
- [ ] `cargo build --all-features` clean
- [ ] `cargo tree --no-default-features` does not list `metrics` — `cargo tree --no-default-features 2>&1 | grep -c '^metrics '` is 0
- [ ] `cargo test` passes
- [ ] `cargo test --features metrics-adapter` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `cargo publish --dry-run` succeeds
- [ ] No new file over 350 lines
- [ ] `gitleaks detect` clean
- [ ] `examples/observability` runs end-to-end — `cargo run --example observability --features metrics-adapter` exits 0; `curl -s localhost:9000/metrics | grep cache_hits_total` returns a line
- [ ] `cd site && npm run build` clean (includes `audit-versions.mjs`)
- [ ] Built site lists `/v0.9.2/integration/observability` and `/latest/integration/observability` as static routes
- [ ] `/integration/observability` returns 308 to `/latest/integration/observability`
- [ ] `CHANGELOG.md` `[0.9.2]` includes observability bullet, docs-versioning bullet, and a Migration subsection
- [ ] `VERSION` and `Cargo.toml` agree on `0.9.2`

---

## Eval Commands (Post-Implementation Verification)

```bash
# Crate
cargo build --release 2>&1 | tail -3
cargo build --no-default-features 2>&1 | tail -3
cargo tree --no-default-features 2>&1 | grep -c '^metrics '   # Expected: 0
cargo build --all-features 2>&1 | tail -3
cargo test 2>&1 | tail -5
cargo test --features metrics-adapter 2>&1 | tail -5
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -3
cargo fmt --check
cargo publish --dry-run 2>&1 | tail -10
gitleaks detect 2>&1 | tail -3
git diff --name-only origin/main | grep -E '\.rs$' | xargs wc -l 2>/dev/null | awk '$1 > 350 { print "OVER: " $2 ": " $1 }'
grep '^version = ' Cargo.toml; cat VERSION

# End-to-end example
cargo run --example observability --features metrics-adapter & EX=$!; sleep 2
curl -s localhost:9000/metrics | grep -E 'cache_(hits|misses|errors)_total'
kill $EX

# Docs site
cd site && npm run build 2>&1 | tail -10
npm run build 2>&1 | grep -E '/v0\.9\.2/|/latest/'
node scripts/audit-versions.mjs
```

---

## Dead Code Sweep

N/A — no files deleted. `src/observability.rs` becomes a re-export shim; existing site pages are MOVED into `site/content/v0.9.2/`, not deleted (the move is tracked by `git mv`).

---

## Discovery (consult log)

> **Empty at creation.** Append as the work surfaces consults and decisions — the spec's running record where deferrals and skill outcomes are proven.

- **Consults** — {Architecture / Legacy-Design / gate-flag triage: question asked + Indy's decision.}
- **Skill chain outcomes** — {`/write-unit-test`, `/review`, `/review-pr`, `kishore-babysit-prs` results: iteration counts, findings dispositioned.}
- **Deferrals** — every "deferred to follow-up" needs an Indy-acked verbatim quote here, format `> Indy (YYYY-MM-DD HH:MM): "<quote>" — context: <which item, why>`. An agent-unilateral deferral is incomplete scope, not deferral, and blocks CHORE(close).

---

## Skill-Driven Review Chain (mandatory)

| When | Skill | Notes |
|------|-------|------|
| After implementation, before CHORE(close) | `/write-unit-test` | Audit diff coverage vs Test Specification. Particular focus: every Failure Modes row has a corresponding negative test. |
| After tests pass, still before CHORE(close) | `/review` | Adversarial diff review against this spec, `RUST_GUIDELINES.txt`, `docs/greptile-learnings/RULES.md`. Focus: trait additivity, feature-flag hygiene, zero-allocation in `StatsCollector`, no PII leakage by default, redirect correctness, audit script coverage. |
| After `gh pr create` opens the PR | `/review-pr` | Re-review the now-immutable diff. Focus: clippy strictness, doc-comment completeness on every new pub item, span/event-name stability, version-selector UX. |

---

## Verification Evidence

> Filled in during VERIFY phase.

| Check | Command | Result | Pass? |
|-------|---------|--------|-------|
| Default build | `cargo build --release` | {pending} | |
| No-default-features build | `cargo build --no-default-features` | {pending} | |
| `metrics` absent from default tree | `cargo tree --no-default-features \| grep -c '^metrics '` | {pending} | |
| All-features build | `cargo build --all-features` | {pending} | |
| Unit tests | `cargo test` | {pending} | |
| Tests with metrics-adapter | `cargo test --features metrics-adapter` | {pending} | |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | {pending} | |
| Fmt | `cargo fmt --check` | {pending} | |
| Publish dry-run | `cargo publish --dry-run` | {pending} | |
| Example end-to-end | example + `curl /metrics` | {pending} | |
| Site build | `cd site && npm run build` | {pending} | |
| Versioned routes prerendered | `npm run build \| grep -E '/v0\.9\.2/\|/latest/'` | {pending} | |
| Version audit | `node site/scripts/audit-versions.mjs` | {pending} | |
| 350L gate | `wc -l` on new `.rs` files | {pending} | |
| Gitleaks | `gitleaks detect` | {pending} | |

---

## Out of Scope

- **OpenTelemetry first-party adapter.** Wire `tracing-opentelemetry` downstream; cache-kit emits `tracing` and is OTel-ready by transitivity.
- **StatsD / Datadog / NewRelic first-party adapters.** Route via `metrics`-crate exporter ecosystem.
- **Per-entity-type sub-counters in `CacheStatsSnapshot`.** Flat for v0.9.2; bucketed snapshots are v1.0.
- **Histogram in built-in `StatsCollector`.** Use the `metrics`-crate adapter; HDR-histogram in v1.0.
- **Circuit breaker** (`plans/comprehensive-code-review.md` §16 #1). Adjacent concern; own spec (M093 candidate). Error codes (M092_001) + stats from this spec are prerequisites.
- **Cache warming, compression, tagging, multi-level helper, RocksDB/DynamoDB backends** — each is its own spec; none belong in v0.9.2.
- **Documentation comparison page vs moka/cached/redis-rs.** This spec replaces the prior M092_002 docs-only scope with library work that gives such a page a real story to tell. A comparison page can revive in v0.9.3 once observability has shipped and the matrix shows ✅ in cache-kit's column for the rows that matter.
- **Cross-version docs search**, **automatic version banners**, **deep-link migration tables.** Each is a small additional spec on top of §7's foundation.
- **Backfilling a `v0.9.0/` content tree.** v0.9.2 is the first versioned snapshot; we don't author historical docs retroactively. Users on v0.9.0 keep using crates.io README + docs.rs.
