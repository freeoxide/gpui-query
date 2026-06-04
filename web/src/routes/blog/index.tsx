import { createFileRoute, Link } from "@tanstack/react-router";
import { getBlogPosts } from "#/lib/content";
import type { BlogFrontmatter } from "#/lib/content";

export const Route = createFileRoute("/blog/")({
  validateSearch: (search: Record<string, unknown>) => ({
    tag: (search.tag as string) || undefined,
  }),
  loader: () => {
    return getBlogPosts();
  },
  head: () => ({
    meta: [
      { title: "Blog - gpui-query" },
      {
        name: "description",
        content: "Articles about async state management in GPUI with gpui-query",
      },
      { property: "og:title", content: "Blog - gpui-query" },
      {
        property: "og:description",
        content: "Articles about async state management in GPUI with gpui-query",
      },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
      { name: "twitter:title", content: "Blog - gpui-query" },
      {
        name: "twitter:description",
        content: "Articles about async state management in GPUI with gpui-query",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/blog" }],
  }),
  component: BlogIndex,
});

function BlogIndex() {
  const posts = Route.useLoaderData();
  const { tag } = Route.useSearch();

  const allTags = Array.from(new Set(posts.flatMap((p) => p.frontmatter.tags ?? []))).sort();

  const filteredPosts = tag ? posts.filter((p) => p.frontmatter.tags?.includes(tag)) : posts;

  return (
    <div className="mx-auto max-w-4xl px-4 py-8 sm:px-6">
      <header className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Blog</h1>
        <p className="mt-2 text-muted-foreground">
          Articles about async state management in GPUI with gpui-query
        </p>
      </header>

      <nav className="mb-8 flex flex-wrap gap-2" aria-label="Filter by tag">
        <Link
          to="/blog"
          search={{ tag: undefined }}
          className={`inline-flex items-center rounded-full px-3 py-1 text-xs font-medium transition-colors ${
            !tag
              ? "bg-primary text-primary-foreground"
              : "bg-muted text-muted-foreground hover:bg-muted/80"
          }`}
        >
          All
        </Link>
        {allTags.map((t) => (
          <Link
            key={t}
            to="/blog"
            search={{ tag: t === tag ? undefined : t }}
            className={`inline-flex items-center rounded-full px-3 py-1 text-xs font-medium transition-colors ${
              t === tag
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:bg-muted/80"
            }`}
          >
            {t}
          </Link>
        ))}
      </nav>

      {filteredPosts.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-border py-16 text-center">
          <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-muted">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="text-muted-foreground"
            >
              <path d="M4 22h16a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2H8a2 2 0 0 0-2 2v16a2 2 0 0 1-2 2Zm0 0a2 2 0 0 1-2-2v-9c0-1.1.9-2 2-2h2" />
              <path d="M18 14h-8" />
              <path d="M15 18h-5" />
              <path d="M10 6h8v4h-8V6Z" />
            </svg>
          </div>
          <h3 className="text-lg font-semibold">No posts found</h3>
          <p className="mt-1 text-sm text-muted-foreground">
            {tag
              ? `There are no posts tagged "${tag}". Try selecting a different tag.`
              : "There are no blog posts yet."}
          </p>
          {tag && (
            <Link
              to="/blog"
              search={{ tag: undefined }}
              className="mt-4 inline-flex items-center rounded-full bg-primary/10 px-4 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
            >
              Clear filter
            </Link>
          )}
        </div>
      ) : (
        <div className="grid gap-6 sm:grid-cols-2">
          {filteredPosts.map((post) => (
            <BlogCard key={post.slug} post={post} activeTag={tag} />
          ))}
        </div>
      )}
    </div>
  );
}

function BlogCard({
  post,
  activeTag,
}: {
  post: { slug: string; frontmatter: BlogFrontmatter };
  activeTag?: string;
}) {
  const gradientAngle = hashString(post.slug) % 360;

  return (
    <Link
      to="/blog/$slug"
      params={{ slug: post.slug }}
      className="group rounded-lg border bg-card text-card-foreground shadow-sm transition-all duration-200 hover:-translate-y-1 hover:shadow-md"
    >
      <div
        className="h-40 rounded-t-lg bg-gradient-to-br from-primary/20 via-primary/5 to-transparent"
        style={{
          background: `linear-gradient(${gradientAngle}deg, hsl(var(--primary) / 0.2), hsl(var(--primary) / 0.05), transparent)`,
        }}
      >
        <div className="flex h-full items-end p-4">
          <time
            dateTime={post.frontmatter.date}
            className="rounded-md bg-background/80 px-2 py-1 text-xs font-medium text-muted-foreground backdrop-blur-sm"
          >
            {new Date(post.frontmatter.date).toLocaleDateString("en-US", {
              year: "numeric",
              month: "short",
              day: "numeric",
            })}
          </time>
        </div>
      </div>

      <div className="p-4">
        <h2 className="text-lg font-semibold leading-snug tracking-tight transition-colors group-hover:text-primary">
          {post.frontmatter.title}
        </h2>
        <p className="mt-2 line-clamp-2 text-sm text-muted-foreground">
          {post.frontmatter.description}
        </p>

        {post.frontmatter.tags && post.frontmatter.tags.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-1.5">
            {post.frontmatter.tags.map((tag: string) => (
              <span
                key={tag}
                className={`inline-flex items-center rounded-full bg-primary/10 px-2.5 py-0.5 text-xs font-medium text-primary transition-colors ${
                  tag === activeTag ? "ring-1 ring-primary/30" : ""
                }`}
              >
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>
    </Link>
  );
}

function hashString(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash |= 0;
  }
  return Math.abs(hash);
}
