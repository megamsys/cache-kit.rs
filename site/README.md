# cache-kit docs site

Next.js 16 + Netlify documentation site for [cache-kit](https://github.com/megamsys/cache-kit.rs),
served at [cachekit.org](https://cachekit.org).

Replaced the prior Jekyll (Minimal Mistakes) build under `docs/`. Mirrors the stack and
design system at [`docs.megam.io`](https://docs.megam.io) — Fraunces / General Sans /
JetBrains Mono on a dark canvas, one cyan accent, mono uppercase eyebrows. The canonical
design spec lives at `~/Projects/docs.megam.io/DESIGN.md`.

---

## TL;DR

```bash
cd site
npm install
npm run dev          # http://127.0.0.1:3000
npm run build        # production build into .next/
```

To add a doc page, drop an `.mdx` file under `content/<section>/<slug>.mdx` with YAML
frontmatter. The sidebar, routing, and metadata regenerate automatically on the next
build.

---

## Stack

| Layer | Choice | Why |
|---|---|---|
| Framework | Next.js 16 (App Router) | Same as `docs.megam.io` — single template to maintain. |
| React | 19.2.6 | Matches Next.js 16. |
| Hosting | Netlify | Auto-deploy on push via `netlify.toml` + `@netlify/plugin-nextjs`. |
| Content | `.mdx` files with YAML frontmatter | Plain markdown bodies; no JSX runtime (yet). |
| Rendering | Hand-rolled parser in `lib/content.ts` | Zero markdown deps. Covers paragraphs, headings, lists, fenced code, tables, blockquotes, images, inline links/code/emphasis. **No remark/rehype, no MDX components.** |
| Styling | Plain CSS in `app/globals.css` | Tokens from DESIGN.md, no Tailwind. |
| TypeScript | 6, strict | — |

If you need real MDX (JSX inside markdown), swap to `@next/mdx` and drop the parser.
For now, the parser is intentionally tiny — easy to read, easy to extend.

---

## File tree

```
site/
├── app/
│   ├── globals.css            ← design tokens + every selector
│   ├── layout.tsx             ← sticky header, side-nav shell, footer
│   ├── page.tsx               ← homepage (hero + section index)
│   ├── not-found.tsx          ← 404
│   └── [section]/[slug]/
│       └── page.tsx           ← dynamic route, static-generated per .mdx
├── content/
│   ├── getting-started/       (4 pages)
│   ├── concepts/              (3 pages)
│   ├── integration/           (3 pages)
│   └── guides/                (3 pages)
├── lib/
│   └── content.ts             ← frontmatter parser, markdown→HTML, sidebar
├── public/
│   └── favicon.ico
├── scripts/
│   └── port-jekyll.mjs        ← one-time Jekyll → MDX migration (kept for history; can delete)
├── netlify.toml               ← build config, cache headers, Next.js plugin pin
├── next.config.mjs
├── package.json
├── tsconfig.json
└── eslint.config.mjs
```

---

## Adding a doc page

1. Pick a section folder under `content/` (or add a new one — see below).
2. Create `<slug>.mdx` with this frontmatter:

   ```mdx
   ---
   title: "Cache invalidation"
   order: 4
   description: "Invalidating cache entries safely under concurrent writes"
   ---
   # Body starts here

   Regular markdown — paragraphs, **bold**, `code`, [links](/concepts/concepts), lists,
   tables, fenced ```rust blocks, blockquotes, images.
   ```

3. The page is reachable at `/<section>/<slug>` (e.g. `/concepts/cache-invalidation`).
4. It appears in the sidebar automatically; `order` controls the position within the
   section. The page title used everywhere comes from the `title:` field.

That's it. No manual route wiring, no sidebar config to edit.

### Linking between pages

Use absolute `/section/slug` paths inside markdown links: `[see installation](/getting-started/installation)`.
Relative or Jekyll-style permalinks (`/installation/`) won't resolve.

### What the parser handles

| Markdown | Status |
|---|---|
| Headings `#`–`######` | ✅ |
| Paragraphs | ✅ |
| `- ` / `* ` unordered lists | ✅ |
| `1. ` ordered lists | ✅ |
| Fenced code ```` ``` ```` with language tag | ✅ |
| Pipe tables (first row = `<th>`) | ✅ |
| Blockquotes `>` | ✅ |
| `---` horizontal rule | ✅ |
| Inline `**bold**`, `*em*`, `` `code` `` | ✅ |
| `[label](href)` links, `![alt](src)` images | ✅ |
| Nested lists | ❌ |
| HTML inside markdown | ❌ (escaped) |
| JSX / MDX components | ❌ (no JSX runtime) |
| Footnotes, task lists, autolinks | ❌ |

If you need anything in the ❌ column, either work around it in markdown or graduate
to `@next/mdx`.

## Adding a new section

Sections are declared in `lib/content.ts`:

```ts
const sectionTitles: Record<string, string> = {
  "getting-started": "Getting Started",
  concepts: "Core Concepts",
  // add your section id + display title here
};

const sectionOrder = ["getting-started", "concepts", /* …, your-section */];
```

Then create `content/<your-section>/` and drop pages in.

---

## How it builds

- `lib/content.ts` walks `content/` at build time, parses frontmatter + body, and
  exports `getSidebar()`, `getAllRoutes()`, `loadPage()`.
- `app/[section]/[slug]/page.tsx` calls `generateStaticParams()` against `getAllRoutes()`
  → Next.js statically generates one HTML file per page (SSG).
- `app/layout.tsx` calls `getSidebar()` on every page so the sidebar is consistent
  without client-side fetching.
- The whole site is static; Netlify serves it from CDN. There's no runtime Node
  server in the request path beyond Next's edge handlers.

`npm run build` output should show ~16 routes prerendered (homepage + 13 pages + 404
+ section index).

---

## Deploy

Already configured. On every push to `main`, Netlify:
1. Runs `npm install`
2. Runs `npm run build` (from `site/` as the base directory)
3. Publishes `.next/` via `@netlify/plugin-nextjs`
4. Applies the cache headers in `netlify.toml`

Pinned via `netlify.toml`: Node 24, no telemetry, no audit/fund noise on install.

### First-time wiring

If you're standing the Netlify site up from scratch:

1. Create the Netlify site (UI or `netlify sites:create --name cache-kit-docs`).
2. Link the GitHub repo. **Base directory:** `site/`. Build: `npm run build`. Publish:
   `.next`.
3. Add custom domain `cachekit.org` (Domain management → Add).
4. Move DNS records at the registrar to Netlify's values (Netlify shows them in the
   domain panel). Let's Encrypt cert auto-provisions once DNS resolves.

See `../docs/CUTOVER.md` for the full GitHub-Pages-removal checklist.

---

## Design system

Theme is locked to the system documented in `~/Projects/docs.megam.io/DESIGN.md`.
Do not, without an entry in the Decisions Log:

- Add a second display font (Fraunces is the voice).
- Add a second accent color (cyan only).
- Introduce bubble `border-radius` (squared corners are part of the register).
- Apply `text-shadow: var(--glow-primary)` to anything other than `.glow-link`.

Edit `app/globals.css` for theme work. All design tokens live in the `:root` block at
the top.

---

## Common pitfalls

| Symptom | Cause | Fix |
|---|---|---|
| Page exists but doesn't appear in sidebar | Missing `order:` in frontmatter, or section not in `sectionOrder` | Add `order: N` to frontmatter or add section to `lib/content.ts`. |
| Link from one doc to another 404s | Used a relative or Jekyll permalink | Use absolute `/section/slug`. |
| Code block renders as a paragraph | Fence opens with extra spaces or wrong tag (`” ```` `` ”`) | Start fence at column 0 with `` ``` ``. |
| Table renders as text | Header row not followed by a `|---|---|` separator | Add the separator row. |
| Fonts flash unstyled (FOUT) | Loaded via `@import` from Google Fonts + Fontshare | Acceptable for now; migrate to `next/font` if it becomes painful. |
| `npm run build` fails on a single page | Frontmatter malformed or unclosed code fence | Run `npm run build` locally; the error names the file. |

---

## History

- May 12, 2026 — Migrated from Jekyll (Minimal Mistakes, GitHub Pages) to Next.js 16
  + Netlify. Content ported via `scripts/port-jekyll.mjs`. See `../docs/CUTOVER.md`.
