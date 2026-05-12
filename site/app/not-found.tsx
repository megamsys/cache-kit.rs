import Link from "next/link";

export default function NotFound() {
  return (
    <article className="content-page">
      <h1>404 — Page not found</h1>
      <p>
        That page doesn&rsquo;t exist. Try the <Link href="/">documentation index</Link> or open an
        issue on{" "}
        <a href="https://github.com/megamsys/cache-kit.rs/issues">GitHub</a>.
      </p>
    </article>
  );
}
