import Link from "next/link";
import { getSidebar } from "@/lib/content";

export default function HomePage() {
  const sidebar = getSidebar();

  return (
    <>
      <section className="home-hero">
        <p className="eyebrow">cache-kit &middot; v0.9.0</p>
        <h1>Async caching boundaries for Rust services.</h1>
        <p>
          cache-kit places <strong>clear cache boundaries</strong> between your database and
          application logic. Backend-agnostic, ORM-agnostic, runtime-friendly — designed to live
          beside your stack, not own it.
        </p>
        <p>
          Works with SQLx, SeaORM, Diesel, tokio-postgres. Swap Redis, Memcached, or InMemory
          without rewriting cache code.
        </p>
        <div className="cta-row">
          <Link className="cta-primary" href="/getting-started/installation">
            Get started
          </Link>
          <a href="https://github.com/megamsys/cache-kit.rs">View on GitHub</a>
        </div>
      </section>

      {sidebar.map((section) => (
        <section key={section.id} className="home-section">
          <h2>{section.title}</h2>
          <ul>
            {section.pages.map((page) => (
              <li key={page.slug}>
                <Link href={page.route}>{page.title}</Link>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </>
  );
}
