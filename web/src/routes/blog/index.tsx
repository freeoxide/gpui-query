import { createFileRoute, Link } from "@tanstack/react-router";
import { getAllBlogPosts } from "#/lib/blog";
import { Badge } from "#/components/ui/badge";
import { ArrowRight, CalendarIcon, UserIcon } from "lucide-react";

export const Route = createFileRoute("/blog/")({
  head: () => ({
    meta: [
      { title: "Blog - gpui-query" },
      {
        name: "description",
        content: "Announcements, deep dives, and updates about gpui-query.",
      },
      { property: "og:title", content: "Blog - gpui-query" },
      {
        property: "og:description",
        content: "Announcements, deep dives, and updates about gpui-query.",
      },
      { property: "og:type", content: "website" },
      { property: "og:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "Blog - gpui-query" },
      {
        name: "twitter:description",
        content: "Announcements, deep dives, and updates about gpui-query.",
      },
      { name: "twitter:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.freeoxide.com/blog" }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify({
          "@context": "https://schema.org",
          "@type": "Blog",
          name: "gpui-query Blog",
          url: "https://gpui-query.freeoxide.com/blog",
          description: "Announcements, deep dives, and updates about gpui-query.",
          blogPost: getAllBlogPosts().map((post) => ({
            "@type": "BlogPosting",
            headline: post.frontmatter.title,
            description: post.frontmatter.description,
            datePublished: post.frontmatter.date,
            author: { "@type": "Person", name: post.frontmatter.author },
            url: `https://gpui-query.freeoxide.com/blog/${post.slug}`,
          })),
        }),
      },
    ],
  }),
  component: BlogPage,
});

function BlogPage() {
  const posts = getAllBlogPosts();

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-16 lg:px-8">
      <div className="mx-auto max-w-3xl">
        {/* Header */}
        <div className="border-l-4 border-primary pl-5">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Blog</h1>
          <p className="mt-2 text-lg text-muted-foreground">
            Announcements, deep dives, and updates about gpui-query.
          </p>
        </div>

        {/* Posts */}
        {posts.length === 0 ? (
          <div className="mt-16 text-center">
            <p className="text-muted-foreground">No posts yet. Check back soon!</p>
          </div>
        ) : (
          <div className="mt-14 space-y-8">
            {posts.map((post, index) => (
              <article
                key={post.slug}
                className="group rounded-xl border border-border/60 bg-card p-6 transition-colors hover:border-primary/20 hover:bg-card/80 sm:p-8"
                style={{
                  animation: `fade-in-up 0.5s ease-out ${index * 100}ms both`,
                }}
              >
                {/* Meta row */}
                <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                  <span className="inline-flex items-center gap-1.5">
                    <CalendarIcon className="h-3.5 w-3.5" />
                    <time dateTime={post.frontmatter.date}>
                      {new Date(post.frontmatter.date).toLocaleDateString("en-US", {
                        year: "numeric",
                        month: "long",
                        day: "numeric",
                      })}
                    </time>
                  </span>
                  {post.frontmatter.author && (
                    <span className="inline-flex items-center gap-1.5">
                      <UserIcon className="h-3.5 w-3.5" />
                      {post.frontmatter.author}
                    </span>
                  )}
                  {index === 0 && (
                    <Badge className="border-transparent bg-primary/10 text-xs font-bold uppercase tracking-wider text-primary">
                      Latest
                    </Badge>
                  )}
                </div>

                {/* Title */}
                <h2 className="mt-3 text-xl font-semibold tracking-tight transition-colors group-hover:text-primary sm:text-2xl">
                  <Link to="/blog/$slug" params={{ slug: post.slug }}>
                    {post.frontmatter.title}
                  </Link>
                </h2>

                {/* Excerpt */}
                {post.frontmatter.description && (
                  <p className="mt-2 leading-relaxed text-muted-foreground">
                    {post.frontmatter.description}
                  </p>
                )}

                {/* Read more */}
                <div className="mt-4">
                  <Link
                    to="/blog/$slug"
                    params={{ slug: post.slug }}
                    className="inline-flex items-center gap-1.5 text-sm font-medium text-primary transition-colors hover:text-primary/80"
                  >
                    Read more
                    <ArrowRight className="h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5" />
                  </Link>
                </div>
              </article>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
