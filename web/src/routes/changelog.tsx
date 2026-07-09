import { createFileRoute } from "@tanstack/react-router";
import { Badge } from "#/components/ui/badge";
import { changelogPage } from "#/lib/seo";

/* ─── Changelog Data ────────────────────────────────────────────── */
/*
 * Mirrors the crate's CHANGELOG.md at the repo root. That file is the source
 * of truth — when a release lands, copy its entry here. Never invent
 * versions, dates, or features that aren't in CHANGELOG.md.
 */

type Category = "Added" | "Changed" | "Fixed" | "Removed";

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
  Removed: { bullet: "-", color: "text-rose-600 dark:text-rose-400" },
};

const changelogEntries: ChangelogEntry[] = [
  {
    version: "v0.1.4",
    date: "2026-06-17",
    description: "Crate metadata and README improvements on crates.io.",
    items: [
      {
        category: "Added",
        text: "Author metadata and an Author section in both crate READMEs",
      },
      {
        category: "Added",
        text: "readme field on gpui-query-legacy so its README renders on crates.io",
      },
    ],
  },
  {
    version: "v0.1.3",
    date: "2026-06-14",
    description: "Decoupled the legacy crate and fixed gpui version compatibility.",
    items: [
      {
        category: "Changed",
        text: "gpui-query-legacy is fully decoupled from the main crate, with standalone docs, tests, and improved hook error handling; it publishes independently",
      },
      { category: "Added", text: "Crate-level README for gpui-query on crates.io" },
      {
        category: "Fixed",
        text: "read_with calls in the hook module are now source-compatible across gpui versions",
      },
    ],
    links: [{ label: "Migrating from v1 to v2", slug: "/docs/advanced/migration" }],
  },
  {
    version: "v0.1.2",
    date: "2025-06-13",
    description: "Single-workflow releases and an independent legacy crate.",
    items: [
      {
        category: "Changed",
        text: "CI publishes both crates in one workflow run, covering tag, GitHub Release, crates.io publish, and website deploy",
      },
      {
        category: "Changed",
        text: "gpui-query-legacy is an independent crate on crates.io; the legacy feature flag and re-export were removed from gpui-query",
      },
      {
        category: "Changed",
        text: "Legacy crate publish step tolerates 'already uploaded' errors on workflow re-runs",
      },
    ],
  },
  {
    version: "v0.1.1",
    date: "2025-06-12",
    description: "The v2 rewrite became the main crate; v1 lives on as gpui-query-legacy.",
    items: [
      {
        category: "Changed",
        text: "The v2 rewrite at crates/gpui-query-v2 is now the main crate; the v1 code moved to crates/gpui-query-legacy",
      },
      {
        category: "Fixed",
        text: "read_with calls in the hook module returning Result instead of the raw value from AsyncApp context (9 call sites)",
      },
      {
        category: "Added",
        text: "#![deprecated] on the legacy crate, with a README pointing to v2",
      },
      { category: "Added", text: "Real content on all 12 documentation pages" },
      {
        category: "Removed",
        text: "All gpui_query_v2 references in source and docs, replaced with gpui_query",
      },
    ],
    links: [{ label: "Migrating from v1 to v2", slug: "/docs/advanced/migration" }],
  },
  {
    version: "v0.1.0",
    date: "2025-06-10",
    description: "Initial public release: core query system, client registry, and GPUI hooks.",
    items: [
      {
        category: "Added",
        text: "QueryResource reactive state container with the QueryStatus lifecycle (Idle, Loading, Success, Failure)",
      },
      {
        category: "Added",
        text: "CachePolicy (TTL, Stale-While-Revalidate), RequestPolicy, and RetryPolicy with configurable backoff",
      },
      {
        category: "Added",
        text: "QuerySignal cooperative cancellation via Arc<AtomicBool>",
      },
      {
        category: "Added",
        text: "QueryClient global registry with typed buckets, observers, and garbage collection",
      },
      {
        category: "Added",
        text: "use_query, use_mutation, and use_infinite_query hooks for GPUI components",
      },
      {
        category: "Added",
        text: "Experimental options-first v2 rewrite with QueryPersister, use_query_select, and ClientDiagnostic devtools",
      },
    ],
    links: [
      { label: "Getting Started", slug: "/docs/getting-started/installation" },
      { label: "Queries", slug: "/docs/api/queries" },
    ],
  },
];

/* ─── Route ─────────────────────────────────────────────────────── */

export const Route = createFileRoute("/changelog")({
  head: () => ({
    meta: [
      { title: "Changelog - gpui-query" },
      {
        name: "description",
        content:
          "gpui-query release history: the v1 to v2 rewrite, crate reorganization, fixes, and docs updates in every published version.",
      },
      { property: "og:title", content: "Changelog - gpui-query" },
      {
        property: "og:description",
        content:
          "gpui-query release history: the v1 to v2 rewrite, crate reorganization, fixes, and docs updates in every published version.",
      },
      { property: "og:type", content: "website" },
      { property: "og:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "Changelog - gpui-query" },
      {
        name: "twitter:description",
        content:
          "gpui-query release history: the v1 to v2 rewrite, crate reorganization, fixes, and docs updates in every published version.",
      },
      { name: "twitter:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.freeoxide.com/changelog" }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify(
          changelogPage({
            name: "gpui-query Changelog",
            description:
              "Release history for gpui-query, the async state management library for GPUI.",
            url: "https://gpui-query.freeoxide.com/changelog",
            softwareVersion: changelogEntries[0].version.replace(/^v/, ""),
            datePublished: changelogEntries[changelogEntries.length - 1].date,
            dateModified: changelogEntries[0].date,
          }),
        ),
      },
    ],
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
