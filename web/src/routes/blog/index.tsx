import { createFileRoute, Link } from "@tanstack/react-router";
import { getBlogPosts } from "#/lib/content";
import type { BlogFrontmatter } from "#/lib/content";
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "#/components/ui/card";

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
    ],
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

      <div className="grid gap-6">
        {filteredPosts.map((post) => (
          <BlogCard key={post.slug} post={post} />
        ))}
        {filteredPosts.length === 0 && (
          <p className="text-muted-foreground">No posts found{tag ? ` with tag "${tag}"` : ""}.</p>
        )}
      </div>
    </div>
  );
}

function BlogCard({ post }: { post: { slug: string; frontmatter: BlogFrontmatter } }) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <time dateTime={post.frontmatter.date}>
            {new Date(post.frontmatter.date).toLocaleDateString("en-US", {
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </time>
        </div>
        <CardTitle className="text-xl">
          <Link
            to="/blog/$slug"
            params={{ slug: post.slug }}
            className="hover:text-primary transition-colors"
          >
            {post.frontmatter.title}
          </Link>
        </CardTitle>
        <CardDescription>{post.frontmatter.description}</CardDescription>
      </CardHeader>
      {post.frontmatter.tags && post.frontmatter.tags.length > 0 && (
        <CardContent>
          <div className="flex flex-wrap gap-2">
            {post.frontmatter.tags.map((tag: string) => (
              <Link
                key={tag}
                to="/blog"
                search={{ tag }}
                className="inline-flex items-center rounded-full bg-primary/10 px-3 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
              >
                {tag}
              </Link>
            ))}
          </div>
        </CardContent>
      )}
    </Card>
  );
}
