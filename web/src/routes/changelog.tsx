import { createFileRoute } from "@tanstack/react-router";
import { Badge } from "#/components/ui/badge";

/* ─── Changelog Data ────────────────────────────────────────────── */

type Category = "Added" | "Changed" | "Fixed";

interface ChangelogSubItem {
  category: Category;
  text: string;
}

interface ChangelogLink {
  label: string;
  slug: string;
}

interface ChangelogEntry {
  version: string;
  date: string;
  description: string;
  items: ChangelogSubItem[];
  links?: ChangelogLink[];
}

const categoryConfig: Record<Category, { bullet: string; color: string }> = {
  Added: { bullet: "+", color: "text-emerald-600 dark:text-emerald-400" },
  Changed: { bullet: "~", color: "text-amber-600 dark:text-amber-400" },
  Fixed: { bullet: "!", color: "text-sky-600 dark:text-sky-400" },
};

const changelogEntries: ChangelogEntry[] = [
  {
    version: "v0.3.0",
    date: "2026-05-15",
    description: "Added infinite queries, persistence API, and expanded diagnostics.",
    items: [
      { category: "Added", text: "Infinite query support with bidirectional fetching" },
      { category: "Added", text: "Persistence API with custom serialization backends" },
      { category: "Added", text: "Expanded diagnostics and devtools integration" },
      { category: "Changed", text: "Improved cache garbage collection heuristics" },
      { category: "Fixed", text: "Race condition when multiple observers share a query key" },
    ],
    links: [
      { label: "Infinite Queries", slug: "/docs/api/infinite-queries/" },
      { label: "Persistence", slug: "/docs/guides/persistence/" },
    ],
  },
  {
    version: "v0.2.0",
    date: "2026-04-20",
    description: "Introduced mutation system, query observers, and select pattern.",
    items: [
      { category: "Added", text: "First-class mutation system with success/error callbacks" },
      { category: "Added", text: "Query observers for fine-grained subscription" },
      { category: "Added", text: "Select pattern for derived query data" },
      { category: "Changed", text: "Refactored internal registry for lower memory footprint" },
      { category: "Fixed", text: "Stale-While-Revalidate returning stale data on error" },
    ],
    links: [{ label: "Mutations", slug: "/docs/api/mutations/" }],
  },
  {
    version: "v0.1.0",
    date: "2026-03-10",
    description: "Initial release with core query system, caching, and retry policies.",
    items: [
      { category: "Added", text: "Core use_query hook with automatic fetching and caching" },
      { category: "Added", text: "Configurable retry policies with exponential backoff" },
      { category: "Added", text: "TTL and Stale-While-Revalidate cache policies" },
      { category: "Added", text: "Cooperative cancellation via Arc<AtomicBool>" },
    ],
    links: [{ label: "Getting Started", slug: "/docs/getting-started/installation/" }],
  },
];

/* ─── Route ─────────────────────────────────────────────────────── */

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
      { property: "og:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "Changelog - gpui-query" },
      {
        name: "twitter:description",
        content: "Release history and changelog for gpui-query.",
      },
      { name: "twitter:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/changelog" }],
  }),
  component: ChangelogPage,
});

/* ─── Page ──────────────────────────────────────────────────────── */

function ChangelogPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-3xl">
        {/* Header */}
        <div className="mb-12 text-center">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Changelog</h1>
          <p className="mt-3 text-lg text-muted-foreground">
            Release history for gpui-query. Every version, every improvement.
          </p>
        </div>

        {/* Timeline */}
        <div className="relative">
          {/* Continuous vertical line */}
          <div
            aria-hidden="true"
            className="absolute bottom-0 left-[7px] top-0 w-px bg-gradient-to-b from-primary/40 via-primary/20 to-transparent"
          />

          <div className="space-y-12">
            {changelogEntries.map((entry, index) => (
              <ChangelogEntryCard
                key={entry.version}
                entry={entry}
                index={index}
                isLatest={index === 0}
                isOldest={index === changelogEntries.length - 1}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Badge Variant Helper ──────────────────────────────────────── */

function getVersionBadgeClasses(index: number, _total: number): { className: string } {
  if (index === 0) {
    return {
      className: "border-transparent bg-primary text-primary-foreground shadow-sm",
    };
  }
  if (index === 1) {
    return {
      className: "border-transparent bg-secondary text-secondary-foreground",
    };
  }
  return {
    className: "border-transparent bg-muted text-muted-foreground",
  };
}

/* ─── Entry Card ────────────────────────────────────────────────── */

function ChangelogEntryCard({
  entry,
  index,
  isLatest,
}: {
  entry: ChangelogEntry;
  index: number;
  isLatest: boolean;
  isOldest?: boolean;
}) {
  const delay = index * 120;
  const badgeClasses = getVersionBadgeClasses(index, changelogEntries.length);

  return (
    <article
      className="relative pl-8"
      style={{
        animation: `fade-in-up 0.5s ease-out ${delay}ms both`,
      }}
    >
      {/* Timeline dot */}
      <div
        aria-hidden="true"
        className={`absolute left-0 top-1.5 flex h-[15px] w-[15px] items-center justify-center rounded-full border-2 ${
          isLatest
            ? "border-primary bg-primary/20 shadow-[0_0_8px_var(--color-primary)/30%]"
            : "border-primary/30 bg-background"
        }`}
      >
        {isLatest && <div className="h-1.5 w-1.5 rounded-full bg-primary" />}
      </div>

      {/* Card body */}
      <div className="rounded-xl border border-border/60 bg-card p-5 transition-colors hover:border-primary/20 hover:bg-card/80 sm:p-6">
        {/* Header row */}
        <div className="flex flex-wrap items-center gap-3">
          <Badge className={badgeClasses.className}>{entry.version}</Badge>
          {isLatest && (
            <span className="inline-flex items-center rounded-full bg-primary/10 px-2.5 py-0.5 text-[11px] font-bold uppercase tracking-wider text-primary">
              Latest
            </span>
          )}
          <time className="text-sm text-muted-foreground">{entry.date}</time>
        </div>

        {/* Description */}
        <p className="mt-3 leading-relaxed text-foreground/90">{entry.description}</p>

        {/* Categorized sub-items */}
        {entry.items.length > 0 && (
          <div className="mt-4 space-y-1.5 rounded-lg bg-muted/40 px-4 py-3">
            {entry.items.map((item, i) => {
              const config = categoryConfig[item.category];
              return (
                <div key={i} className="flex items-start gap-2 text-sm">
                  <span
                    className={`mt-0.5 shrink-0 font-mono text-xs font-bold leading-none ${config.color}`}
                    aria-hidden="true"
                  >
                    {config.bullet}
                  </span>
                  <span className="text-muted-foreground">
                    <span className={`mr-1.5 font-semibold ${config.color}`}>{item.category}</span>
                    {item.text}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Related links */}
        {entry.links && entry.links.length > 0 && (
          <div className="mt-4 flex flex-wrap gap-x-4 gap-y-1.5 border-t border-border/40 pt-3">
            <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground/70">
              Docs
            </span>
            {entry.links.map((link) => (
              <a
                key={link.slug}
                href={link.slug}
                className="inline-flex items-center text-sm font-medium text-primary transition-colors hover:text-primary/80"
              >
                {link.label}
                <svg
                  className="ml-1 h-3 w-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  strokeWidth={2}
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3"
                  />
                </svg>
              </a>
            ))}
          </div>
        )}
      </div>
    </article>
  );
}
