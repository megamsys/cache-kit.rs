<!--
SPEC AUTHORING RULES (load-bearing — do not delete):
- No time/effort/hour/day estimates anywhere in this spec.
- No effort columns, complexity ratings, percentage-complete, implementation dates.
- No assigned owners — use git history and handoff notes.
- Priority (P0/P1/P2) is the only sizing signal. Use Dependencies for sequencing.
- If a section below contradicts these rules, the rule wins — delete the section.
- See ~/Projects/dotfiles/docs/TEMPLATE.md for the canonical template.
-->

# M092_002: cache-kit vs other Rust caches — comparison page on docs site

**Prototype:** v0.9.2
**Milestone:** M092
**Workstream:** 002
**Date:** May 12, 2026
**Status:** PENDING
**Priority:** P2 — pure docs work; doesn't gate the library release but addresses a long-standing gap (no honest comparison vs moka/cached/redis-rs in user-facing docs).
**Categories:** DOCS
**Batch:** B2 — runs after M092_001 lands the new error surface; this spec doesn't depend on M092_001's code, but ordering keeps v0.9.2 release notes coherent.
**Branch:** {feat/m092-vs-other-caches — added when work begins}
**Depends on:** none (independent of M092_001)
**Provenance:** agent-generated (pre-spec, derived from `plans/comprehensive-code-review.md` §12 #2 and §18)

> The source review (`plans/comprehensive-code-review.md`) contains a 7-row self-rated comparison table in §18 — useful as a starting point but unfit to ship as-is. Two problems with the source: (a) self-ratings ("Documentation ⭐⭐⭐⭐⭐") have no external grounding, and (b) the table doesn't surface moka's actual strengths (TinyLFU eviction, sub-µs in-process reads, no network hop) — so a reader can't tell *when to use moka over cache-kit*. This spec demands the page name those strengths honestly.

**Canonical architecture:** N/A — docs work only. Lives under `site/content/concepts/`.

---

## Implementing agent — read these first

1. `site/lib/content.ts` — the markdown parser that will render this page. Confirms which Markdown features the page can rely on (paragraphs, headings, fenced code, tables, blockquotes, inline links/code/emphasis) and what's unsupported (nested lists, HTML, MDX components).
2. `site/content/concepts/concepts.mdx` and `site/content/concepts/async-model.mdx` — voice and section-shape to mirror. Don't reinvent tone; match what's already shipping.
3. `site/app/[section]/[slug]/page.tsx` — confirms that `/concepts/vs-other-caches` will be a real route once the file lands; no route registration step required.
4. `plans/comprehensive-code-review.md` §12, §18 (intent only) — the recommendation and the seed table. Treat the existing table as **inputs**, not as the deliverable. The shipping table must drop self-ratings and add an honest *"when to pick which"* column.
5. moka's own README (https://github.com/moka-rs/moka) — read it. You cannot write an honest comparison without knowing what moka actually does. Pay attention to: TinyLFU eviction policy, sync vs async APIs, `Cache` vs `SegmentedCache`, expiration policies (TTI vs TTL), notification callbacks.

---

## Applicable Rules

- **`docs/greptile-learnings/RULES.md`** — universal repo discipline; applies to the diff.
- **DESIGN.md system** at `~/Projects/docs.megam.io/DESIGN.md` (canonical for the docs site) — typography, accent, no second display font, no bubble border-radius, mono uppercase for eyebrows. The new page must render consistently with the existing `concepts/*.mdx` pages.
- **`site/README.md` — "What the parser handles" table** — the markdown parser is hand-rolled; nested lists, HTML, MDX components, autolinks are unsupported. Author the page against this constraint.

---

## Overview

**Goal (testable):** `https://cachekit.org/concepts/vs-other-caches` renders an honest, evidence-backed comparison between cache-kit, moka, cached, and redis-rs — where each library is best, where it isn't, when to use both together — and the page passes `cd site && npm run build` cleanly while remaining within the markdown parser's supported feature set.

**Problem:** Users (and search-engine crawlers) cannot find an unbiased "when do I pick cache-kit over moka" answer in cache-kit's own docs. The internal review surfaced this gap (`plans/comprehensive-code-review.md` §12 #2, §18) but its seed table is self-rated and doesn't name moka's actual strengths. A reader who lands here today either picks based on README vibes or gives up. Both are bad outcomes — users who'd be better served by moka adopt cache-kit and hit friction; users who'd be better served by cache-kit adopt moka and hit different friction.

**Solution summary:** A new MDX page at `site/content/concepts/vs-other-caches.mdx`, ordered after `serialization.mdx` in the concepts section. It carries: (a) a re-shot comparison matrix with capability rows (no star ratings); (b) an explicit *"when to pick which"* decision section covering the four most common scenarios (single-instance L1, multi-instance shared cache, embedded/no-network, edge); (c) a worked example showing moka-as-L1 layered with cache-kit-as-L2; (d) a *"things cache-kit deliberately doesn't do"* section calling out moka's actual wins (TinyLFU, sub-µs reads, no-network-hop). Page MUST NOT claim performance numbers that haven't been measured against moka on equivalent inputs.

---

## Files Changed (blast radius)

| File | Action | Why |
|------|--------|-----|
| `site/content/concepts/vs-other-caches.mdx` | CREATE | The deliverable. New page; reachable at `/concepts/vs-other-caches`. |
| `site/lib/content.ts` | EDIT (one line) | If the page exceeds the parser's current capability (e.g. needs nested lists), this spec is wrong — author the page within the parser's limits instead. Touched only if a real gap is found, with a separate decision in the PR description. |
| `CHANGELOG.md` | EDIT | New `## [0.9.2]` section gets a "Docs: added vs-other-caches comparison page" line. (Coordinate with M092_001 — same release section, two bullets.) |
| `plans/comprehensive-code-review.md` §12 / §18 | EDIT (annotation only) | Add a Status column to the §18 table linking to this spec, so the review's finding is marked SPEC'D not OPEN per REVIEW_TEMPLATE.md conventions. This is the spec's only touch on the review doc. |

---

## Sections (implementation slices)

### §1 — Capability matrix (no stars, no self-ratings)

Deliver: a table with rows = capabilities, columns = `cache-kit / moka / cached / redis-rs`, cells = ✅ / ❌ / ⚠️ / N/A (consistent with the parser's table renderer). Rows MUST be objective and verifiable from each library's README/docs — not subjective.

**Implementation default rows:**
- Type-safe API
- Backend-agnostic (in-process + remote)
- In-process only (no network hop)
- Async-first
- Sync-supported
- Eviction policy (LRU / TinyLFU / TTL-only / N/A)
- Versioned serialization envelope
- Multi-backend hot-swap

**Implementation default:** drop the "Documentation" star rating row from the source table. Doc quality is downstream of the reader, not the library.

### §2 — "When to pick which" decision section

Deliver: four short subsections, one per scenario. Each one names the scenario, names the right pick, and gives the one-line reason. The agent picks the wording; the constraint is no scenario can resolve to "use cache-kit" without a concrete reason.

**Scenarios to cover:**
- Single-instance service, hot data ≤ in-process memory.
- Multi-instance service sharing cache state.
- Embedded / single-binary deployment, no Redis available.
- Edge / serverless function with strict cold-start budget.

**Implementation default:** for "single-instance, hot data fits in memory," the answer is **moka** — call it out. cache-kit's value is at the L2 boundary; pretending it beats moka at L1 reads is dishonest and the review's table currently implies that.

### §3 — Worked example: moka-as-L1 + cache-kit-as-L2

Deliver: a code snippet showing the two libraries layered. moka holds the hot working set; cache-kit handles the Redis hop on L1 miss. Short — ≤ 30 lines of Rust. No `cargo run` required; the snippet is illustrative, not executable.

**Implementation default:** keep the snippet to one function. If the agent reads moka's README and finds the layered pattern is awkward in current moka, document that instead — don't fake an idiomatic snippet.

### §4 — "Things cache-kit deliberately doesn't do" section

Deliver: a short prose list naming the work moka does that cache-kit explicitly doesn't (and won't). TinyLFU eviction, lock-free reads via concurrent hash table, sub-µs in-process latency, in-process-only optimization. The point is to *say it out loud* in cache-kit's own docs — readers immediately know whether they're in cache-kit's wheelhouse.

**Implementation default:** end the section with one line: "If you need any of the above, use moka; cache-kit is the wrong tool." That sentence is the page's honest signal.

---

## Interfaces

> A docs page has no API surface in the Rust sense, but it does have a URL contract and a frontmatter contract.

```
Route:    /concepts/vs-other-caches
Source:   site/content/concepts/vs-other-caches.mdx

Frontmatter contract:
---
title: "cache-kit vs other Rust caches"
order: 4
description: "How cache-kit relates to moka, cached, and redis-rs — when to pick which, when to use both."
---
```

`order: 4` places the page after `concepts.mdx` (1), `async-model.mdx` (2), `serialization.mdx` (3) per existing ordering in `site/lib/content.ts`. The sidebar regenerates automatically.

---

## Failure Modes

| Mode | Cause | Handling |
|------|-------|----------|
| Page renders but a table breaks layout | Parser doesn't support a markdown feature the author used (nested cells, `<br>` in cells) | Author against `site/README.md`'s "What the parser handles" table; if a needed feature is missing, file a separate spec to extend the parser instead of working around it. |
| Performance claim has no measurement | Author wrote "cache-kit is 2x faster than X" without running a benchmark | Page review checklist: every quantitative claim must cite a benchmark in `benches/` OR be removed. |
| moka's strengths get hand-waved | Author marketed cache-kit instead of comparing honestly | §4 ("Things cache-kit deliberately doesn't do") is mandatory and cannot be empty. PR review enforces. |
| Page name shows up but `npm run build` fails | Frontmatter malformed or unclosed code fence | `cd site && npm run build` in the Acceptance Criteria catches this pre-PR. |
| Comparison data is wrong | Author misread moka/cached/redis-rs docs | Required prologue step #5 (read moka README) is the safeguard. If the reviewer spots a factual error, treat as a P0 fix-forward in the same PR. |

---

## Invariants

1. **No self-rating ⭐ columns** — the parser supports `⭐` characters but the page MUST NOT use them for cache-kit's own ratings. Enforced by review (greppable: `grep -c '⭐' site/content/concepts/vs-other-caches.mdx` = 0).
2. **§4 is non-empty** — the "deliberately doesn't do" section MUST list at least 3 specific moka capabilities cache-kit doesn't match. Enforced by review.
3. **Every performance claim cites evidence** — if the page says *"X is faster than Y"*, the next sentence cites a benchmark file path in `benches/` or a public benchmark URL. Enforced by review.
4. **Page builds clean** — `cd site && npm run build` produces no errors or warnings related to this file. Enforced by Acceptance Criteria.
5. **Sidebar renders the page in the correct slot** — under "Core Concepts," after Serialization. Visible in the rendered HTML. Enforced by spot-check during VERIFY.

---

## Test Specification

> Docs pages don't have unit tests; verification is the build + a manual spot-check.

| Test | Asserts |
|------|---------|
| `build_includes_page` | `cd site && npm run build` lists `/concepts/vs-other-caches` as a prerendered static route in the output. |
| `frontmatter_parses` | The page's `title`, `order: 4`, and `description` are picked up by `site/lib/content.ts::loadPage("concepts", "vs-other-caches")`. |
| `sidebar_order` | The page appears in the "Core Concepts" sidebar section after "Serialization." |
| `no_self_rating_stars` | `grep -c '⭐' site/content/concepts/vs-other-caches.mdx` returns 0. |
| `section_4_present` | Page contains a section titled "Things cache-kit deliberately doesn't do" (or equivalent — confirmed by grep). |
| `parser_renders_table` | The capability matrix renders as `<table>` with `<th>` headers in the produced HTML (not as escaped text). |

Negative: none worth automating beyond the above. Edge: extremely long table — limit to ≤ 12 rows in §1 to keep mobile layout readable.

---

## Acceptance Criteria

- [ ] `cd site && npm install && npm run build` succeeds with zero errors — verify: command output's last line is `✓ Compiled successfully` or equivalent.
- [ ] Built output lists `/concepts/vs-other-caches` as a static route — verify: `npm run build 2>&1 | grep vs-other-caches`.
- [ ] Page is no more than 350 lines — verify: `wc -l site/content/concepts/vs-other-caches.mdx`.
- [ ] No `⭐` characters in the page — verify: `grep -c '⭐' site/content/concepts/vs-other-caches.mdx` is 0.
- [ ] §4 exists and is non-empty — verify: `grep -A1 "deliberately doesn't do" site/content/concepts/vs-other-caches.mdx` shows substantive prose.
- [ ] `plans/comprehensive-code-review.md` §18 has a "Status / Spec" column added with this spec linked.
- [ ] `CHANGELOG.md` `[0.9.2]` section includes a docs bullet.
- [ ] Spot-check the rendered page on `npm run dev` — sidebar order, table layout, links, mobile collapse.

---

## Eval Commands (Post-Implementation Verification)

```bash
# E1: Build clean
cd site && npm run build 2>&1 | tail -5

# E2: New route prerendered
cd site && npm run build 2>&1 | grep -E 'vs-other-caches|/concepts/'

# E3: No self-rating stars
grep -c '⭐' site/content/concepts/vs-other-caches.mdx
# Expected: 0

# E4: §4 present
grep -i "deliberately doesn't do" site/content/concepts/vs-other-caches.mdx
# Expected: 1+ match

# E5: 350-line gate
wc -l site/content/concepts/vs-other-caches.mdx
# Expected: <= 350

# E6: Frontmatter valid
head -5 site/content/concepts/vs-other-caches.mdx
# Expected: title, order: 4, description present

# E7: Review doc updated
grep -A1 "M092_002" plans/comprehensive-code-review.md
# Expected: §18 row references this spec
```

---

## Dead Code Sweep

N/A — no files deleted. Purely additive.

---

## Skill-Driven Review Chain (mandatory)

| When | Skill | Notes |
|------|-------|------|
| After implementation, before CHORE(close) | `/write-unit-test` | N/A — docs page has no unit tests. Skill returns clean by inspection; record the N/A in Ripley's Log. |
| After tests pass, still before CHORE(close) | `/review` | Adversarial review against the Invariants table (no self-ratings, §4 non-empty, claims cited). Particular focus: did the author market cache-kit, or compare honestly? |
| After `gh pr create` opens the PR | `/review-pr` | Re-review focused on factual accuracy of the moka/cached/redis-rs claims; flag anything the author got wrong about another library. |

---

## Verification Evidence

> Filled in during VERIFY phase.

| Check | Command | Result | Pass? |
|-------|---------|--------|-------|
| Build clean | `cd site && npm run build` | {pending} | |
| Route prerendered | `npm run build 2>&1 \| grep vs-other-caches` | {pending} | |
| No self-ratings | `grep -c '⭐' ...` | {pending} | |
| §4 present | `grep -i "deliberately doesn't do" ...` | {pending} | |
| 350L gate | `wc -l ...` | {pending} | |

---

## Out of Scope

- **Running benchmarks vs moka.** The page can cite existing benchmarks from `benches/` but is not the right place to do new perf work. New perf comparison is its own spec — the work involves picking workloads and committing CI compute.
- **Migration guide from moka → cache-kit.** Different spec, different deliverable. The §12 #3 review recommendation ("migration guide from other caching solutions") is a separate page that requires real migration examples, not just a comparison table.
- **Updating the README's elevator pitch.** The README's tagline shouldn't depend on this page existing. Drift between README and this page is fine for v0.9.2.
- **The other 10 review recommendations** (circuit breaker, observability, cache warming, RocksDB/DynamoDB backends, compression, tagging, statistics, multi-level, video tutorials, performance benchmarks page). Each gets its own spec when picked up. Most are library work, not docs work, and don't belong in v0.9.2's patch scope.
