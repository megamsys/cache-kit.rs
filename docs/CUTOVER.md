# GitHub Pages → Netlify cutover

The cache-kit documentation site lived as a Jekyll (Minimal Mistakes) build under
`docs/`, deployed to `cachekit.org` via `.github/workflows/pages.yml`. The new site is
a Next.js 16 + Netlify build under `site/` mirroring the stack at
[`docs.megam.io`](https://docs.megam.io).

This document is the ordered cutover checklist. Each step is reversible until step 4.

## Pre-flight

- [ ] Verify content parity: every file under `docs/_pages/` (and `docs/index.md`) has a
      ported `.mdx` under `site/content/<section>/<slug>.mdx`. Re-run
      `node site/scripts/port-jekyll.mjs` if you edit any source markdown.
- [ ] Read both files side-by-side for the migrated pages. The lightweight markdown
      parser does not support Jekyll-specific features (`{% include %}`, kramdown attr
      lists, `has_children: true`); the porter strips them, but cross-check headings,
      tables, and code blocks.
- [ ] Local build: `cd site && npm install && npm run build` succeeds with zero errors.

## 1. Stand up the Netlify site

- [ ] Create the Netlify site (UI or `netlify sites:create --name cache-kit-docs`).
- [ ] Connect this repo. Base directory: `site/`. Build: `npm run build`. Publish:
      `.next`. Node version 24 (pinned in `site/netlify.toml`).
- [ ] Trigger a deploy. Confirm the auto-generated preview URL (e.g.
      `https://cache-kit-docs.netlify.app`) renders the homepage, sidebar, and at least
      one nested page (`/getting-started/installation`).
- [ ] Open browser devtools → Network. Confirm Fraunces / General Sans / JetBrains Mono
      load (no FOUT, no 404s).

## 2. Move the apex domain

`cachekit.org` is currently pointed at GitHub Pages via the `docs/CNAME` file plus DNS
A-records for the GitHub Pages apex (185.199.108.153 + 109/110/111). To switch:

- [ ] In Netlify → Domain management → Add custom domain → `cachekit.org`.
- [ ] At your DNS provider, replace the GitHub Pages A-records with Netlify's
      values (Netlify will display them; typically a Netlify load-balancer A-record
      plus `www` CNAME to `<site>.netlify.app`).
- [ ] Wait for DNS to propagate (usually <15 min, can be up to 24h). Verify with
      `dig cachekit.org +short` — should resolve to the Netlify load balancer.
- [ ] In Netlify, click "Verify DNS configuration". Netlify will provision Let's Encrypt
      automatically once DNS resolves.
- [ ] Confirm `https://cachekit.org` serves the new site.

## 3. Stop the GitHub Pages build

Even with DNS pointed away, the Pages workflow still rebuilds on every push to `main`
and the `gh-pages` environment still exists. Clean it up:

- [ ] Delete `.github/workflows/pages.yml`.
- [ ] GitHub → Settings → Pages → Source → **None**. (This also disables the
      `github-pages` environment.)
- [ ] Optional: delete the `gh-pages` branch if one exists (`git push origin --delete
      gh-pages`).

## 4. Remove Jekyll source

Once the Netlify site is serving traffic on the apex domain, delete the Jekyll source.
This is the point of no return — make sure step 2 is verified.

```bash
# from repo root
git rm docs/CNAME docs/_config.yml docs/Gemfile docs/Gemfile.lock \
       docs/favicon.ico docs/index.md docs/README.md
git rm -r docs/_data docs/_pages
git rm -rf docs/_site  # Jekyll build artifact, may not exist locally
git commit -m "docs: remove Jekyll source after Netlify cutover"
```

**Keep** the agent-rules symlinks under `docs/` — they're consumed by `AGENTS.md` and
are not part of the website build:

```
docs/BUN_RULES.md            -> dotfiles/docs/BUN_RULES.md
docs/gates/                  -> dotfiles/docs/gates/
docs/greptile-learnings/     -> dotfiles/docs/greptile-learnings/
docs/LIFECYCLE_PATTERNS.md   -> dotfiles/docs/LIFECYCLE_PATTERNS.md
docs/LOGGING_STANDARD.md     -> dotfiles/docs/LOGGING_STANDARD.md
docs/REST_API_DESIGN_GUIDELINES.md -> dotfiles/docs/REST_API_DESIGN_GUIDELINES.md
docs/SCHEMA_CONVENTIONS.md   -> dotfiles/docs/SCHEMA_CONVENTIONS.md
docs/TEMPLATE.md             -> dotfiles/docs/TEMPLATE.md
docs/ZIG_RULES.md            -> dotfiles/docs/ZIG_RULES.md
docs/ZIG_STATIC_OPENSSL.md   -> dotfiles/docs/ZIG_STATIC_OPENSSL.md
```

Also keep `site/scripts/port-jekyll.mjs` for now (in case a stray markdown file needs
re-porting from git history) — delete it after the next docs change lands cleanly.

## 5. Post-cutover housekeeping

- [ ] Update `README.md` at repo root: replace any link to the Jekyll docs build with
      `https://cachekit.org`.
- [ ] Update `CHANGELOG.md` with a brief note: *"docs: migrated from GitHub Pages /
      Jekyll to Netlify / Next.js (cachekit.org unchanged)"*.
- [ ] If any external references hardcode the GitHub Pages URL
      (`megamsys.github.io/cache-kit.rs/`), set up a redirect in Netlify:
      `[[redirects]] from="https://megamsys.github.io/cache-kit.rs/*"
      to="https://cachekit.org/:splat" status=301 force=true` — although Pages will
      404 by then anyway.

## Rollback

If something goes wrong between steps 2 and 4:

1. Restore the GitHub Pages A-records at DNS.
2. Re-enable Pages: Settings → Pages → Source → Deploy from branch → `main` / `/docs`.
3. Revert the deletion commit.

After step 4, rollback requires restoring from git history — same revert, but the
workflow run needs to succeed against a working Jekyll tree.
