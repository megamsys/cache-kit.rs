<!--
SPEC AUTHORING RULES (load-bearing — do not delete):
- No time/effort/hour/day estimates anywhere in this spec.
- No effort columns, complexity ratings, percentage-complete, implementation dates.
- No assigned owners — use git history and handoff notes.
- Priority (P0/P1/P2) is the only sizing signal. Use Dependencies for sequencing.
- If a section below contradicts these rules, the rule wins — delete the section.
- See ~/Projects/dotfiles/docs/TEMPLATE.md for the canonical template.
-->

# M092_001: Structured error codes and context for `cache_kit::Error`

**Prototype:** v0.9.2
**Milestone:** M092
**Workstream:** 001
**Date:** May 12, 2026
**Status:** PENDING
**Priority:** P1 — production consumers (Actix/Axum services) cannot programmatically distinguish failure modes from string messages; blocks first-class API integration and monitoring.
**Categories:** API · OBS · DOCS
**Batch:** B1
**Branch:** {feat/m092-error-codes — added when work begins}
**Depends on:** none
**Provenance:** agent-generated (pre-spec, derived from `plans/error-codes-improvement.md`)

> The source proposal in `plans/error-codes-improvement.md` is an 850-line design document containing inlined Rust pseudocode (pinned enum values, full match arms, sample test bodies). This spec is the goal-contract conversion — the implementing agent reads the proposal for intent but writes the implementation from project conventions in `src/error.rs`, not from the proposal's code blocks.

**Canonical architecture:** N/A — no `docs/ARCHITECTURE.md` in this repo. Implementation surface is `src/error.rs` and its callers across `src/`.

---

## Implementing agent — read these first

1. `src/error.rs` (224 lines) — current `Error` enum, `From` impls, `Display` impl. This is the pattern to mirror for error variants. Note the existing per-variant doc comments documenting "Common causes" and "Recovery"; preserve that voice.
2. `src/expander.rs` — primary consumer of `Error`. Look at every `Err(Error::...)` construction site to understand which call sites need updating to carry codes + context.
3. `src/backend/redis.rs` and `src/backend/memcached.rs` — error-mapping from the backend libraries into `cache_kit::Error`. The new error code surface must distinguish backend-source errors from validation/serialization errors at the call site.
4. `plans/error-codes-improvement.md` (intent only — do NOT copy code) — read **Problem Statement**, **Benefits**, and **Migration Strategy** sections. Ignore everything inside ``` ``` ``` ``` fences; the agent writes Rust per project conventions, not per the proposal's example.
5. `examples/actixsqlx/src/services/user_service.rs` — a real consumer showing how errors propagate to an HTTP response. The new error surface must make this consumer's code simpler, not more verbose.

---

## PR Intent & comprehension handshake

> The bridge from spec to the merged PR. Makes the agent confirm it understood intent *before* writing code.

- **PR title (eventual):** `feat(error): add structured ErrorCode + ErrorContext to cache_kit::Error`
- **Intent (one sentence):** A service-layer consumer can match on a stable error code to dispatch retry-vs-fail-vs-surface without parsing `Display` strings, and existing v0.9.x consumers rebuild unchanged.
- **Handshake (agent fills at PLAN, before EXECUTE):** the implementing agent restates the intent in its own words and lists `ASSUMPTIONS I'M MAKING: …`. A mismatch between that restatement and the Intent above → STOP and reconcile before any edit.

---

## Applicable Rules

- **Project rule source:** `RUST_GUIDELINES.txt` at repo root (90KB) — read sections covering error handling, public API stability, and trait bounds.
- **`docs/greptile-learnings/RULES.md`** — universal repo discipline; applies to the diff.
- **Semver discipline** — this is a v0.9.x change; the public `Error` enum is part of the consumed API. Changes must be additive (new variants, new methods) for v0.9.2. Removing or renaming existing variants is a v1.0 concern and out of scope here.
- **Postcard envelope versioning** — if any error variant is serialized into a cache envelope (it shouldn't be — errors aren't cached), the envelope version rules in `src/serialization/` apply.

---

## Applicable Gates

> Which Action-Triggered Guards this PR WILL trip, and how each stays clean. Rules ≠ Gates: rules are knowledge to read; gates fire on edits.

| Gate | Fires? | Satisfaction strategy |
|------|--------|-----------------------|
| File & Function Length (≤350/≤50/≤70) | yes — `src/error.rs` is already 224 lines and grows | Split `ErrorCode` into `src/error/code.rs` and `ErrorContext` into `src/error/context.rs` (already in Files Changed); keep each new file well under 350. |
| PUB / Struct-Shape | yes — new `ErrorCode` enum, `ErrorContext` struct, methods on `Error` | Additive only; every existing public name keeps its signature. Doc-comment every new `pub` item per the existing per-variant "Common causes / Recovery" voice. |
| UFS (repeated/semantic literals) | yes — `"E{:04}"` format + numeric ranges | Named constants for the format width and each range boundary; no magic numbers scattered across call sites. |
| ERROR REGISTRY | yes — this spec *defines* the code registry | Codes live in one enum with an exhaustive `Error::code()` match; numeric values pinned by golden fixture (Invariant 1). |
| LOGGING | no | No new log call sites; errors are returned, not logged, by this surface. |
| SCHEMA / ZIG / UI / DESIGN TOKEN | no | No `*.sql`, `*.zig`, or UI surface touched. |

---

## Overview

**Goal (testable):** Every `cache_kit::Error` value carries (a) a stable machine-readable code drawn from a documented numeric range, (b) a retryability hint, (c) an HTTP status mapping, and (d) optional context (cache key, operation name) — and a service-layer consumer can match on the code to dispatch retry vs fail vs surface-to-client without parsing strings.

**Problem:** Today every `Error` variant is `Variant(String)`. Production consumers (Actix services, Axum handlers, gRPC streams in the `examples/` tree) have three observable problems:
1. Cannot distinguish "Redis is down" (retry with backoff) from "user submitted bad UUID" (return 400) without string-matching, which is fragile across versions.
2. Cannot emit per-error-class metrics (Prometheus counter by code) — every error counts as "one of N possible string flavors."
3. Cannot translate user-facing error responses without parsing the message — i18n is impossible.

**Solution summary:** Add an `ErrorCode` enum keyed by stable `u32` values in documented ranges, plus an `ErrorContext` struct carrying cache key / operation. Existing `Error` variants gain methods (`code()`, `context()`, `is_retryable()`, `http_status()`) without changing their tuple shape, so the change is additive in v0.9.2. Each variant's code is set deterministically by its kind; context is opt-in via builder methods. Consumers see a richer surface without breaking changes.

---

## Prior-Art / Reference Implementations

> Mirror a known-good pattern instead of inventing.

- **In-repo pattern to mirror:** `src/error.rs` — the existing `Error` enum, its `From` impls, and the per-variant doc-comment voice ("Common causes" / "Recovery"). The new `code()` mapping mirrors the existing `Display` impl's exhaustive match structure.
- **Stable-code precedent:** `std::io::ErrorKind` (a flat, non-exhaustive enum mapping error kinds to a stable, matchable surface) is the canonical Rust shape for "match the kind, not the message." `ErrorCode` follows it, adding numeric stability + HTTP mapping.
- **HTTP-status mapping precedent:** `http::StatusCode` constants — `http_status()` returns the raw `u16` rather than taking an `http` crate dependency (cache-kit stays framework-agnostic; consumers map the `u16` into their own framework's status type).
- **Divergence:** unlike `io::ErrorKind`, `ErrorCode` pins each variant to a documented `u32` (golden-tested) because the value is a wire/observability contract, not just an in-process discriminant.

---

## Files Changed (blast radius)

| File | Action | Why |
|------|--------|-----|
| `src/error.rs` | EDIT | Add `ErrorCode` enum, `ErrorContext` struct, methods on `Error`, builder pattern. Existing variants preserved. |
| `src/error/code.rs` | CREATE | Houses `ErrorCode` enum + its impls (`as_u32`, `as_string`, `description`, `is_retryable`, `http_status`). Split out to keep `error.rs` under 350 lines. |
| `src/error/context.rs` | CREATE | Houses `ErrorContext` struct + builder methods. |
| `src/error.rs` callers across `src/` | EDIT | Update every `Err(Error::...)` site to wire the new error code. Estimated 25–40 sites; agent grep-counts before editing. |
| `tests/error_codes_test.rs` | CREATE | Coverage for code-stability, retryability truth-table, HTTP-status mapping, context propagation, response-serialization shape. |
| `CHANGELOG.md` | EDIT | New `## [0.9.2]` section describing the additive surface. |
| `Cargo.toml` | EDIT | Bump `version = "0.9.2"`. |
| `VERSION` | EDIT | `0.9.2`. |
| `site/content/integration/error-handling.mdx` | CREATE | User-facing error catalog page (replaces the proposal's `docs/_pages/error-catalog.md` plan). Reachable at `/integration/error-handling`. |

---

## Decomposition & alternatives (patch vs refactor)

> Match solution-size to problem-size; surface the call before approval.

- **Chosen shape:** additive surface — new `ErrorCode`/`ErrorContext` types + accessor methods on the existing `Error`, with a single internal `variant → code` mapping. Existing tuple shapes are frozen. Split across `error/code.rs` + `error/context.rs` to respect the length gate.
- **Alternatives considered:**
  1. *Full reshape to `Variant { code, context, message }` structs (the v1.0 refactor).* Rejected for v0.9.2 — it breaks every existing `Error::SerializationError(s)` pattern match, making it a breaking change that belongs behind a major version.
  2. *Minimal patch: a free function `error_code(&Error) -> u32` and nothing else.* Rejected — leaves retryability/HTTP-status as caller homework, so it doesn't actually solve the "dispatch retry-vs-fail-vs-surface" goal; consumers still string-match for the rest.
- **Patch-vs-refactor verdict:** this is a **patch** (additive, no shape change) because the goal is reachable without breaking consumers. The full reshape is the right long game and is named explicitly in **Out of Scope** as the v1.0 follow-up.

---

## Sections (implementation slices)

### §1 — `ErrorCode` enum + numeric ranges

Deliver: a `pub enum ErrorCode` with `#[repr(u32)]` whose variants cover every existing `Error` variant kind, organized into documented numeric ranges. Each variant has `as_u32()`, `as_string()` (e.g. `"E1000"`), `description()` (static prose), `is_retryable() -> bool`, and `http_status() -> u16`.

**Implementation default:** numeric ranges grouped by category (serialization, validation, backend, repository, config, operation). The agent picks the exact ranges; the constraint is they're contiguous, leave headroom for additions, and are documented in the user-facing error catalog page (§4).

**Implementation default:** `is_retryable()` returns `true` for backend/network/timeout codes; `false` for validation/serialization. The agent reads `src/expander.rs::retry_with_backoff` to confirm the retryability semantics align with what the existing retry loop expects.

### §2 — `ErrorContext` struct + builder methods on `Error`

Deliver: a `pub struct ErrorContext` carrying optional `cache_key: Option<String>` and `operation: Option<String>`. Add builder methods on `Error`: `with_cache_key(self, key)`, `with_operation(self, op)`. Each existing `Error` variant gains a `context()` accessor returning `&ErrorContext`.

**Implementation default:** keep context internal-only — do NOT add it as a tuple field on existing variants. Use a side-channel (e.g. wrap or extend the existing inner string + a separate context). The constraint is **the existing `Error` enum's tuple shapes do not change** in v0.9.2; otherwise this is a breaking change requiring v1.0. The agent picks the storage shape that satisfies that constraint.

### §3 — Wire codes through existing call sites

Deliver: every `Err(Error::...)` construction in `src/` produces an error with the correct code already attached, without callers having to write `Error::Foo("...").with_code(ErrorCode::Bar)`. The mapping from variant → code is internal; callers just construct variants as before.

**Implementation default:** map variant → code in a single function (`Error::code(&self) -> ErrorCode`). No per-call-site annotation. The agent reads the existing `From<RedisError> for Error` (and Memcached equivalent) to keep backend-source errors mapped to the right code range.

### §4 — User-facing error catalog page

Deliver: a new MDX page under `site/content/integration/error-handling.mdx` with frontmatter (`title`, `order: 4`, `description`) listing every error code, its description, retryability, and HTTP status. Auto-generated from the `ErrorCode` enum if practical; hand-written if the parser/build pipeline can't support codegen.

**Implementation default:** hand-written for v0.9.2. The agent confirms the markdown parser in `site/lib/content.ts` renders the catalog cleanly (table support exists). Codegen is a v1.0 concern.

---

## Interfaces

> Lock the contract. Public-API additions only — every existing public name keeps its existing signature.

```
pub enum ErrorCode {
    // Range partitions documented in §1; exact values are the agent's call within the partition.
    // Example partition (illustrative — not pinned):
    //   1000-1099: Serialization
    //   1200-1299: Validation
    //   1400-1499: Backend (Redis, Memcached, network)
    //   1500-1599: Repository (DB)
    //   1700-1799: Operation (timeout, cancelled, retry exhausted)
}

impl ErrorCode {
    pub fn as_u32(self) -> u32;
    pub fn as_string(self) -> String;       // "E1000"
    pub fn description(self) -> &'static str;
    pub fn is_retryable(self) -> bool;
    pub fn http_status(self) -> u16;
}

pub struct ErrorContext {
    pub cache_key: Option<String>,
    pub operation: Option<String>,
}

impl Error {
    pub fn code(&self) -> ErrorCode;
    pub fn context(&self) -> &ErrorContext;
    pub fn with_cache_key(self, key: impl Into<String>) -> Self;
    pub fn with_operation(self, op: impl Into<String>) -> Self;
}

// Existing variants unchanged.
```

**Out of contract:** wire format. `to_error_response()` / JSON serialization of errors is out of scope for v0.9.2 — that's a v1.0 concern when we have a settled error-wire-format. v0.9.2 ships only the in-process Rust surface.

---

## Failure Modes

| Mode | Cause | Handling |
|------|-------|----------|
| `Error::code()` called on a variant with no mapped code | Implementation bug — new variant added without updating the mapping | Compile-time error via `match` exhaustiveness on the variant → code mapping. The agent uses `match` (not `match _ =>`) so missing variants fail to compile. |
| `with_cache_key` called twice on the same error | Last write wins is fine; no panic | Document in the method's doc-comment. |
| Context strings contain PII (e.g. raw user emails as cache keys) | Operator concern, not framework concern | Document in the error-handling page: cache keys are surfaced in error logs; sanitize sensitive prefixes upstream. |
| Caller relies on `Error::SerializationError(String)` tuple shape via pattern match | Existing v0.9.x consumer code | Variant shape unchanged; the addition is methods + new types. Existing patterns keep working. Verified by adding a regression test. |
| New variant added in v0.9.3 without a code mapping | Forgetting to extend the mapping | Compile-time error (exhaustive match). Caught by `cargo build`. |

---

## Invariants

1. **Code stability** — once an `ErrorCode` variant ships in a release, its `as_u32()` value never changes. Enforced via a golden-blob test in `tests/error_codes_test.rs` that asserts each code's numeric value against a pinned fixture. Renaming the Rust variant is fine; changing its `u32` value fails the test.
2. **Variant exhaustiveness in `Error::code()`** — every `Error` variant maps to exactly one `ErrorCode`. Enforced by `match`-exhaustiveness; a new variant without a mapping fails to compile.
3. **Tuple-shape preservation** — every existing `Error` variant's tuple/struct shape is identical to v0.9.0. Enforced by a regression test that pattern-matches `Error::SerializationError(_)`, `Error::VersionMismatch { expected: _, found: _ }`, etc. for every existing variant.
4. **`is_retryable()` matches the retry loop's expectation** — `src/expander.rs::retry_with_backoff` retries iff `error.code().is_retryable()`. Enforced by a unit test that drives the retry loop with each code and asserts retry-count behavior.
5. **HTTP status codes are valid RFC 7231 values** — `http_status()` always returns 400, 404, 408, 500, 502, 503, or 504 — never 0, never a 1xx/2xx/3xx. Enforced by a unit test iterating all `ErrorCode` variants.

---

## Test Specification

> Prose-and-assertions only. The implementing agent writes the actual test code in project style.

| Test | Asserts |
|------|---------|
| `test_error_code_stable_numeric_values` | Every variant's `as_u32()` matches the pinned `tests/fixtures/error_codes.golden` snapshot. Renaming the Rust variant doesn't change the number. |
| `test_error_code_as_string_format` | `ErrorCode::SerializationFailed.as_string() == "E1000"` (or whatever the agent picked) — format is `"E{:04}"`, zero-padded to 4 digits. |
| `test_error_code_description_nonempty` | Every variant returns a non-empty `description()` string. Catches drop-through `_ => ""` regressions. |
| `test_is_retryable_truth_table` | Backend / network / timeout codes return `true`; validation / serialization / not-found codes return `false`. Iterates all variants against a pinned classification fixture. |
| `test_http_status_is_valid_response_status` | Every variant's `http_status()` is in {400, 404, 408, 500, 502, 503, 504}. |
| `test_error_with_context_round_trip` | `Error::SerializationError("x").with_cache_key("user:123").with_operation("get").context()` returns context with both fields set. |
| `test_error_variant_tuple_shape_preserved` | For every existing variant, a pattern match (`Error::SerializationError(s)`, `Error::VersionMismatch { expected, found }`, etc.) still compiles and binds the expected types. Regression guard. |
| `test_existing_From_impls_attach_correct_code` | `<Error as From<RedisError>>::from(red_err).code() == ErrorCode::RedisOperationFailed` (or appropriate variant). Same for Memcached + Postcard. |
| `test_retry_loop_uses_is_retryable` | Drive `CacheExpander::strategy_refresh` with a stub backend returning each code; assert retry count matches `is_retryable()` for that code. Integration-style test in `tests/`. |
| `test_existing_error_string_format_unchanged` | The `Display` output of `Error::SerializationError("foo")` matches the v0.9.0 format byte-for-byte. Catches accidental message-format drift. |

**Negative tests** match Failure Modes table 1:1. **Edge cases:** empty string in `with_cache_key`, multi-byte chars in operation name, very long context strings.

---

## Acceptance Criteria

- [ ] `cargo build --release` clean — verify: `cargo build --release 2>&1 | tail -3`
- [ ] `cargo test` passes (all 193+ existing tests + new error-code tests) — verify: `cargo test 2>&1 | tail -5`
- [ ] `cargo clippy --all-targets -- -D warnings` clean — verify: `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --check` clean — verify: `cargo fmt --check`
- [ ] `cargo publish --dry-run` succeeds — verify: `cargo publish --dry-run 2>&1 | tail -10`
- [ ] No file over 350 lines added — verify: `git diff --name-only origin/main | grep -E '\.(rs|mdx)$' | xargs wc -l 2>/dev/null | awk '$1 > 350'`
- [ ] `gitleaks detect` clean — verify: `gitleaks detect 2>&1 | tail -3`
- [ ] The new error-handling page renders cleanly on the site build — verify: `cd site && npm run build 2>&1 | tail -5`
- [ ] `CHANGELOG.md` has a `## [0.9.2]` section listing the additive surface.
- [ ] `VERSION` and `Cargo.toml` agree on `0.9.2`.

---

## Eval Commands (Post-Implementation Verification)

```bash
# E1: Build clean
cargo build --release 2>&1 | tail -3

# E2: All tests pass
cargo test 2>&1 | tail -5

# E3: New error-code tests run
cargo test error_codes 2>&1 | grep "test result" | head -5

# E4: Lint clean
cargo clippy --all-targets -- -D warnings 2>&1 | tail -3

# E5: Format clean
cargo fmt --check

# E6: Crate publishes (dry run)
cargo publish --dry-run 2>&1 | tail -5

# E7: 350-line gate (exempts .md / .mdx)
git diff --name-only origin/main | grep -E '\.(rs)$' | xargs wc -l 2>/dev/null | awk '$1 > 350 { print "OVER: " $2 ": " $1 " lines (limit 350)" }'

# E8: Gitleaks
gitleaks detect 2>&1 | tail -3

# E9: Site build (error-handling page renders)
cd site && npm run build 2>&1 | tail -5

# E10: Version sync — Cargo.toml + VERSION agree on 0.9.2
grep '^version = ' Cargo.toml; cat VERSION
```

---

## Dead Code Sweep

N/A — no files deleted. This is additive surface only.

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
| After tests pass, still before CHORE(close) | `/review` | Adversarial diff review against this spec, `RUST_GUIDELINES.txt`, and `docs/greptile-learnings/RULES.md`. Focus areas: backwards-compatibility (existing match patterns must keep compiling), no inadvertent `unsafe`, no `panic!` paths. |
| After `gh pr create` opens the PR | `/review-pr` | Re-review the now-immutable diff. Focus areas: clippy strictness, doc-comment completeness on the new public surface. |

---

## Verification Evidence

> Filled in during VERIFY phase.

| Check | Command | Result | Pass? |
|-------|---------|--------|-------|
| Unit tests | `cargo test` | {pending} | |
| Clippy | `cargo clippy --all-targets -- -D warnings` | {pending} | |
| Fmt | `cargo fmt --check` | {pending} | |
| Publish dry-run | `cargo publish --dry-run` | {pending} | |
| Site build | `cd site && npm run build` | {pending} | |
| 350L gate | `wc -l` on new `.rs` files | {pending} | |
| Gitleaks | `gitleaks detect` | {pending} | |

---

## Out of Scope

- **Wire format / JSON serialization of errors** (`to_error_response`, `ErrorResponse` struct). Deferred to v1.0 alongside a settled wire format. v0.9.2 ships in-process Rust types only.
- **Breaking changes to existing `Error` variant shapes.** Deferred to v1.0. v0.9.2 is purely additive.
- **Internationalization / translation tables for error descriptions.** Out of scope; the proposal's "i18n" benefit is delivered indirectly by stable codes (downstream can build their own translation map).
- **Auto-generation of the error catalog page from the `ErrorCode` enum.** Hand-authored for v0.9.2; codegen is a v1.0 concern.
- **Removing the existing `Error::Other(String)` catch-all.** Deferred to v1.0; for now it gets code `ErrorCode::Unknown`.
- **Other recommendations from `plans/comprehensive-code-review.md`** (circuit breaker, observability hooks, cache warming, etc.) — each one gets its own spec when picked up.
