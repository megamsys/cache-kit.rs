// Port the Jekyll `_pages/` collection at ../docs/_pages into site/content/
// Mirrors docs.megam.io's port-jekyll.mjs approach (stripLiquid, frontmatter
// rewrite, normalize internal links). Run once; idempotent.
//
//   node scripts/port-jekyll.mjs
//
// Maps individual pages into the four-section taxonomy used by lib/content.ts:
//
//   getting-started/  introduction, installation, installation-agents, quick-start
//   concepts/         concepts, async-model, serialization
//   integration/      database-compatibility, api-frameworks, backends
//   guides/           failure-modes, monitoring, troubleshooting

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SITE = path.resolve(__dirname, "..");
const REPO = path.resolve(SITE, "..");
const SRC_PAGES = path.join(REPO, "docs", "_pages");
const SRC_INDEX = path.join(REPO, "docs", "index.md");
const OUT = path.join(SITE, "content");

// Source-of-truth: where each Jekyll page lands and in what order.
const MAP = [
  // [source rel to docs/, dest section, dest slug, order]
  ["index.md", "getting-started", "introduction", 1],
  ["_pages/installation.md", "getting-started", "installation", 2],
  ["_pages/installation-agents.md", "getting-started", "installation-agents", 3],
  ["_pages/guides/quick-start-request-lifecycle.md", "getting-started", "quick-start", 4],

  ["_pages/concepts.md", "concepts", "concepts", 1],
  ["_pages/async-model.md", "concepts", "async-model", 2],
  ["_pages/serialization.md", "concepts", "serialization", 3],

  ["_pages/database-compatibility.md", "integration", "database-compatibility", 1],
  ["_pages/api-frameworks.md", "integration", "api-frameworks", 2],
  ["_pages/backends.md", "integration", "backends", 3],

  ["_pages/guides/failure-modes.md", "guides", "failure-modes", 1],
  ["_pages/guides/monitoring.md", "guides", "monitoring", 2],
  ["_pages/guides/troubleshooting.md", "guides", "troubleshooting", 3]
];

function stripLiquid(src) {
  src = src.replace(/\{%[\s\S]*?%\}/g, "");
  src = src.replace(/\{\{[\s\S]*?\}\}/g, "");
  // kramdown attribute lists: standalone {: .info} lines and inline { .btn .btn-primary }
  src = src.replace(/^\s*\{:[^}]*\}\s*$/gm, "");
  src = src.replace(/\s*\{:[^}]*\}/g, "");
  // kramdown inline button-style attrs after links: [Get Started](installation){ .btn .btn-primary }
  src = src.replace(/\]\(([^)]+)\)\{[^}]*\}/g, "]($1)");
  return src;
}

function parseFrontmatter(src) {
  if (!src.startsWith("---")) return { meta: {}, body: src };
  const end = src.indexOf("\n---", 3);
  if (end === -1) return { meta: {}, body: src };
  const fm = src.slice(3, end).trim();
  const body = src.slice(end + 4).replace(/^\n/, "");
  const meta = {};
  for (const line of fm.split("\n")) {
    const sep = line.indexOf(":");
    if (sep === -1) continue;
    const k = line.slice(0, sep).trim();
    const v = line.slice(sep + 1).trim().replace(/^"(.*)"$/, "$1");
    meta[k] = v;
  }
  return { meta, body };
}

function emitFrontmatter(meta) {
  const lines = ["---"];
  if (meta.title) lines.push(`title: "${meta.title}"`);
  if (meta.order != null) lines.push(`order: ${meta.order}`);
  if (meta.description) lines.push(`description: "${meta.description}"`);
  lines.push("---", "");
  return lines.join("\n");
}

// Rewrite Jekyll permalink-style internal links (e.g. /installation/, /async-model)
// to the new section/slug routing.
const ROUTE_REMAP = new Map(
  MAP.flatMap(([_src, section, slug]) => {
    const route = `/${section}/${slug}`;
    return [
      [`/${slug}/`, route],
      [`/${slug}`, route],
      [`(${slug})`, `(${route})`]
    ];
  })
);

// Specific aliases used in the source markdown.
const ALIASES = new Map([
  ["installation", "/getting-started/installation"],
  ["installation-agents", "/getting-started/installation-agents"],
  ["concepts", "/concepts/concepts"],
  ["async-model", "/concepts/async-model"],
  ["serialization", "/concepts/serialization"],
  ["database-compatibility", "/integration/database-compatibility"],
  ["api-frameworks", "/integration/api-frameworks"],
  ["backends", "/integration/backends"]
]);

function rewriteLinks(body) {
  // [label](installation) → [label](/getting-started/installation)
  body = body.replace(/\]\(([^)\/][^)]*)\)/g, (m, href) => {
    if (/^https?:/.test(href) || href.startsWith("#")) return m;
    // Trim trailing slash / fragment for lookup
    const [target, frag] = href.split("#");
    const bare = target.replace(/\/$/, "");
    const remap = ALIASES.get(bare);
    return remap ? `](${remap}${frag ? `#${frag}` : ""})` : m;
  });
  // [label](/installation/) and similar absolute Jekyll permalinks
  body = body.replace(/\]\((\/[^)#]+)(#[^)]*)?\)/g, (m, target, frag = "") => {
    const bare = target.replace(/\/$/, "");
    const key = bare.startsWith("/") ? bare.slice(1) : bare;
    const remap = ALIASES.get(key);
    return remap ? `](${remap}${frag})` : m;
  });
  return body;
}

let ported = 0;
for (const [relPath, section, slug, order] of MAP) {
  const srcPath = path.join(REPO, "docs", relPath);
  if (!fs.existsSync(srcPath)) {
    console.warn(`  SKIP (missing): ${relPath}`);
    continue;
  }
  const raw = fs.readFileSync(srcPath, "utf8");
  const { meta, body } = parseFrontmatter(raw);
  let cleaned = stripLiquid(body);
  cleaned = rewriteLinks(cleaned);
  cleaned = cleaned.replace(/\n{3,}/g, "\n\n");
  const outMeta = {
    title: meta.title || slug,
    description: meta.description,
    order
  };
  const dstDir = path.join(OUT, section);
  fs.mkdirSync(dstDir, { recursive: true });
  const dstPath = path.join(dstDir, `${slug}.mdx`);
  fs.writeFileSync(dstPath, emitFrontmatter(outMeta) + cleaned.trim() + "\n");
  console.log(`  ${relPath}  ->  content/${section}/${slug}.mdx`);
  ported++;
}

console.log(`\nPorted ${ported} pages into ${path.relative(SITE, OUT)}/`);
