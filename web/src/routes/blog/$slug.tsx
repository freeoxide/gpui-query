import { createFileRoute } from "@tanstack/react-router";
import { getBlogBySlug } from "#/lib/content";

export const Route = createFileRoute("/blog/$slug")({
  loader: async ({ params: { slug } }) => {
    const post = getBlogBySlug(slug);
    if (!post) throw new Error(`Blog post not found: ${slug}`);
    return post;
  },
  head: ({ loaderData }) => ({
    meta: [
      { title: `${loaderData?.frontmatter.title ?? "Blog"} - gpui-query Blog` },
      { name: "description", content: loaderData?.frontmatter.description ?? "" },
      { property: "og:title", content: loaderData?.frontmatter.title ?? "" },
      {
        property: "og:description",
        content: loaderData?.frontmatter.description ?? "",
      },
      { property: "og:type", content: "article" },
    ],
  }),
  component: BlogPostPage,
});

function BlogPostPage() {
  const post = Route.useLoaderData();
  const Content = post.Content;

  return (
    <article className="mx-auto max-w-3xl px-4 py-8 sm:px-6">
      <header className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">{post.frontmatter.title}</h1>
        <div className="mt-4 flex items-center gap-4 text-sm text-muted-foreground">
          <time dateTime={post.frontmatter.date}>
            {new Date(post.frontmatter.date).toLocaleDateString("en-US", {
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </time>
          {post.frontmatter.readingTime && <span>{post.frontmatter.readingTime} min read</span>}
          {post.frontmatter.tags && post.frontmatter.tags.length > 0 && (
            <>
              <span aria-hidden="true">&middot;</span>
              <div className="flex flex-wrap gap-2">
                {post.frontmatter.tags.map((tag: string) => (
                  <span
                    key={tag}
                    className="inline-flex items-center rounded-full bg-primary/10 px-3 py-1 text-xs font-medium text-primary"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </>
          )}
        </div>
      </header>
      <div className="prose prose-neutral dark:prose-invert max-w-none" data-pagefind-body>
        <Content />
      </div>
      <footer className="mt-12 border-t pt-6">
        <a
          href="/blog"
          className="text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          &larr; Back to all posts
        </a>
      </footer>
    </article>
  );
}
