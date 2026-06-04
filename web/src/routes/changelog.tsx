import { createFileRoute, Link } from "@tanstack/react-router";
import { Badge } from "#/components/ui/badge";

const changelogEntries = [
  {
    version: "v0.3.0",
    date: "2026-05-15",
    description: "Added infinite queries, persistence API, and expanded diagnostics.",
  },
  {
    version: "v0.2.0",
    date: "2026-04-20",
    description: "Introduced mutation system, query observers, and select pattern.",
  },
  {
    version: "v0.1.0",
    date: "2026-03-10",
    description: "Initial release with core query system, caching, and retry policies.",
  },
];

export const Route = createFileRoute("/changelog")({
  head: () => ({
    meta: [
      { title: "Changelog - gpui-query" },
      {
        name: "description",
        content: "Release history and changelog for gpui-query.",
      },
      { property: "og:title", content: "Changelog - gpui-query" },
      {
        property: "og:description",
        content: "Release history and changelog for gpui-query.",
      },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
      { name: "twitter:title", content: "Changelog - gpui-query" },
      {
        name: "twitter:description",
        content: "Release history and changelog for gpui-query.",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/changelog" }],
  }),
  component: ChangelogPage,
});

function ChangelogPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-3xl">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Changelog</h1>
        <p className="mt-4 text-lg text-muted-foreground">Release history for gpui-query.</p>

        <div className="mt-10 space-y-10">
          {changelogEntries.map((entry) => (
            <article key={entry.version} className="relative border-l-2 border-primary/20 pl-6">
              <div className="absolute -left-[9px] top-0 h-4 w-4 rounded-full border-2 border-primary bg-background" />
              <div className="flex items-center gap-3">
                <Badge variant="secondary">{entry.version}</Badge>
                <time className="text-sm text-muted-foreground">{entry.date}</time>
              </div>
              <p className="mt-2 leading-relaxed text-foreground">{entry.description}</p>
              {entry.version === "v0.3.0" && (
                <div className="mt-3 flex flex-wrap gap-2">
                  <Link
                    to="/docs/$slug"
                    params={{ slug: "infinite-queries" }}
                    className="text-sm text-primary hover:underline"
                  >
                    Infinite Queries
                  </Link>
                  <Link
                    to="/docs/$slug"
                    params={{ slug: "persistence" }}
                    className="text-sm text-primary hover:underline"
                  >
                    Persistence
                  </Link>
                </div>
              )}
              {entry.version === "v0.2.0" && (
                <div className="mt-3 flex flex-wrap gap-2">
                  <Link
                    to="/docs/$slug"
                    params={{ slug: "mutations" }}
                    className="text-sm text-primary hover:underline"
                  >
                    Mutations
                  </Link>
                </div>
              )}
              {entry.version === "v0.1.0" && (
                <div className="mt-3 flex flex-wrap gap-2">
                  <Link
                    to="/docs/$slug"
                    params={{ slug: "getting-started" }}
                    className="text-sm text-primary hover:underline"
                  >
                    Getting Started
                  </Link>
                </div>
              )}
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}
