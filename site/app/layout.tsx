import type { Metadata } from "next";
import Link from "next/link";
import "./globals.css";
import { getSidebar } from "@/lib/content";

export const metadata: Metadata = {
  metadataBase: new URL("https://cachekit.org"),
  title: {
    default: "cache-kit — Async caching boundaries for Rust",
    template: "%s | cache-kit"
  },
  description:
    "Async, ORM-agnostic caching boundaries for Rust services. Backend-agnostic, runtime-friendly, explicit by design.",
  alternates: { canonical: "/" },
  robots: { index: true, follow: true },
  openGraph: {
    type: "website",
    siteName: "cache-kit",
    url: "https://cachekit.org",
    title: "cache-kit — Async caching boundaries for Rust",
    description:
      "Async, ORM-agnostic caching boundaries for Rust services. Backend-agnostic, runtime-friendly, explicit by design."
  }
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const sidebar = getSidebar();

  return (
    <html lang="en">
      <body>
        <a className="skip-link" href="#content">
          Skip to content
        </a>
        <header className="site-header" aria-label="Site header">
          <Link className="wordmark glow-link" href="/">
            cache-kit
          </Link>
          <nav className="docs-nav" aria-label="External links">
            <a href="https://github.com/megamsys/cache-kit.rs">GitHub</a>
            <a href="https://crates.io/crates/cache-kit">crates.io</a>
          </nav>
        </header>
        <div className="shell">
          <aside className="side-nav" aria-label="Documentation navigation">
            {sidebar.map((section) => (
              <div key={section.id} className="nav-section">
                <span className="nav-section-title">{section.title}</span>
                {section.pages.map((page) => (
                  <Link key={page.slug} href={page.route}>
                    {page.title}
                  </Link>
                ))}
              </div>
            ))}
          </aside>
          <main id="content" className="page-main">
            {children}
          </main>
        </div>
        <footer className="site-footer" aria-label="Site footer">
          <div className="site-footer-inner">
            <p className="entity">cache-kit &middot; MIT licensed</p>
            <p className="meta">
              Built by{" "}
              <a href="https://github.com/indykish">Kishore Kumar Neelamegam</a> at{" "}
              <a href="https://github.com/megamsys">Megam Systems</a>.
            </p>
            <p className="meta">
              <a href="https://github.com/megamsys/cache-kit.rs/issues">Issues</a> &middot;{" "}
              <a href="https://github.com/megamsys/cache-kit.rs/blob/main/CHANGELOG.md">
                Changelog
              </a>{" "}
              &middot;{" "}
              <a href="https://github.com/megamsys/cache-kit.rs/blob/main/LICENSE">License</a>
            </p>
          </div>
        </footer>
      </body>
    </html>
  );
}
