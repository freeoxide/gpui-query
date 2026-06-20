import { createFileRoute, Link } from "@tanstack/react-router";
import { Badge } from "#/components/ui/badge";
import { getAllBlogPosts } from "#/lib/blog";
import { ArrowRight } from "lucide-react";

/* ─── Route ─────────────────────────────────────────────────────────── */

export const Route = createFileRoute("/blog/")({
  head: () => ({
    meta: [
      { title: "Blog - gpui-query" },
      {
        name: "description",
        content:
          "Articles on gpui-query — async state management, caching, cooperative cancellation, and building editors with GPUI.",
      },
      { property: "og:title", content: "Blog - gpui-query" },
      {
        property: "og:description",
        content:
          "Articles on gpui-query — async state management, caching, and cooperative cancellation for GPUI.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://gpui-query.hmziq.xyz/blog" },
      { property: "og:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "Blog - gpui-query" },
      {
        name: "twitter:description",
        content: "Articles on gpui-query — async state management for GPUI.",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/blog" }],
  }),
  component: BlogIndex,
});

/* ─── Helpers ───────────────────────────────────────────────────────── */

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" });
}

/* ─── Page ──────────────────────────────────────────────────────────── */

function BlogIndex() {
  const posts = getAllBlogPosts();

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-16 lg:px-8">
      <div className="mx-auto max-w-3xl">
        {/* Header */}
        <div className="border-l-4 border-primary pl-5">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Blog</h1>
          <p className="mt-2 text-lg text-muted-foreground">
            Deep dives on gpui-query, GPUI internals, and async state in Rust.
          </p>
        </div>

        {/* Post list */}
        <div className="mt-14 space-y-6">
          {posts.map((post) => (
            <Link
              key={post.slug}
              to="/blog/$slug"
              params={{ slug: post.slug }}
              className="group block rounded-xl border border-border/60 bg-card p-6 transition-colors hover:border-primary/30 hover:bg-card/80"
            >
              <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                <time dateTime={post.frontmatter.date}>{formatDate(post.frontmatter.date)}</time>
                <span aria-hidden="true">·</span>
                <span>{post.frontmatter.author}</span>
              </div>

              <h2 className="mt-2 text-xl font-semibold tracking-tight text-foreground transition-colors group-hover:text-primary">
                {post.frontmatter.title}
              </h2>

              <p className="mt-2 leading-relaxed text-muted-foreground">
                {post.frontmatter.description}
              </p>

              {post.frontmatter.tags && post.frontmatter.tags.length > 0 && (
                <div className="mt-4 flex flex-wrap gap-2">
                  {post.frontmatter.tags.map((tag) => (
                    <Badge key={tag} variant="secondary" className="font-normal">
                      {tag}
                    </Badge>
                  ))}
                </div>
              )}

              <div className="mt-4 inline-flex items-center text-sm font-medium text-primary transition-colors group-hover:text-primary/80">
                Read article
                <ArrowRight className="ml-1.5 h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5" />
              </div>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
