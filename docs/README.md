# cache-kit Documentation

This directory contains the documentation for cache-kit, published to GitHub Pages using Jekyll with the just-the-docs theme.

## Structure

```
docs/
├── _config.yml              # Jekyll configuration
├── Gemfile                  # Ruby dependencies for Jekyll
├── index.md                 # Homepage / Introduction
├── async-model.md           # Async Programming Model
├── concepts.md              # Core Concepts (includes Design Philosophy)
├── installation.md          # Installation & Configuration
├── database-compatibility.md # Database & ORM Compatibility
├── api-frameworks.md        # API Frameworks & Transport Layers
├── serialization.md         # Serialization Support
├── backends.md              # Cache Backend Support
├── guides/                  # User guides
│   ├── index.md
│   └── testing.md
└── reference/               # Technical reference
    └── index.md
```

## Local Development

To run the documentation site locally:

### Prerequisites

- Ruby 3.x or higher
- Bundler

### Setup

```bash
cd docs

# Install dependencies
bundle install

# Run Jekyll server
bundle exec jekyll serve

# View at http://localhost:4000/cache-kit.rs/
```

### Live Reload

Jekyll will automatically rebuild the site when files change. Refresh your browser to see updates.

## Publishing to GitHub Pages

The documentation is automatically published to GitHub Pages when pushed to the `main` branch.

**Published URL:** https://megamsys.github.io/cache-kit.rs/

### GitHub Pages Configuration

1. Go to repository Settings → Pages
2. Source: Deploy from a branch
3. Branch: `main`
4. Folder: `/docs`
5. Save

GitHub will automatically build and deploy the site using Jekyll.

## Theme

This site uses the [just-the-docs](https://just-the-docs.github.io/just-the-docs/) theme.

- **Features:** Search, navigation tree, mobile responsive
- **Customization:** See `_config.yml`
- **Documentation:** https://just-the-docs.github.io/just-the-docs/

## Front Matter

Each markdown file should include Jekyll front matter:

```yaml
---
layout: default
title: Page Title
parent: Parent Section # Optional, for nested pages
nav_order: 1 # Optional, controls navigation order
---
```

## Adding New Pages

1. Create a new `.md` file in the appropriate directory
2. Add front matter (see above)
3. Write content in Markdown
4. The page will automatically appear in navigation

## Syntax Highlighting

Code blocks are automatically syntax highlighted:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

## Links

- **Documentation Site:** https://megamsys.github.io/cache-kit.rs/
- **Repository:** https://github.com/megamsys/cache-kit.rs
- **just-the-docs Docs:** https://just-the-docs.github.io/just-the-docs/
