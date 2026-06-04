import { createFileRoute } from "@tanstack/react-router";
import { getBlogBySlug } from "#/lib/content";
import { blogPost } from "#/lib/seo";

export const Route = createFileRoute("/blog/$slug")({
  loader: ({ params: { slug } }) => {
    const post = getBlogBySlug(slug);
    if (!post) throw new Error(`Blog post not found: ${slug}`);
    // Only return serializable data — Content component is resolved in the component
    return { frontmatter: post.frontmatter, slug };
  },
  head: ({ loaderData, params }) => {
    if (!loaderData) return { meta: [], links: [], scripts: [] };
    const fm = loaderData.frontmatter;
    const slug = params.slug;
    return {
      meta: [
        { title: `${fm.title} - gpui-query Blog` },
        { name: "description", content: fm.description },
        { property: "og:title", content: fm.title },
        { property: "og:description", content: fm.description },
        { property: "og:type", content: "article" },
        {
          property: "og:url",
          content: `https://gpui-query.hmziq.xyz/blog/${slug}`,
        },
        { property: "article:published_time", content: fm.date },
        { name: "twitter:card", content: "summary" },
        { name: "twitter:title", content: fm.title },
        { name: "twitter:description", content: fm.description },
      ],
      links: [
        {
          rel: "canonical",
          href: `https://gpui-query.hmziq.xyz/blog/${slug}`,
        },
      ],
      scripts: [
        {
          type: "application/ld+json",
          children: JSON.stringify(
            blogPost({
              headline: fm.title,
              description: fm.description,
              author: fm.author ?? "hmziq",
              datePublished: fm.date,
              url: `https://gpui-query.hmziq.xyz/blog/${slug}`,
            }),
          ),
        },
      ],
    };
  },
  component: BlogPostPage,
});

function BlogPostPage() {
  const { slug } = Route.useParams();
  const { frontmatter } = Route.useLoaderData();

  // Resolve Content component directly — avoids Seroval serialization failure
  const post = getBlogBySlug(slug);
  if (!post) return null;
  const Content = post.Content;

  return (
    <article className="mx-auto max-w-3xl px-4 py-8 sm:px-6">
      <header className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">{frontmatter.title}</h1>
        <div className="mt-4 flex items-center gap-4 text-sm text-muted-foreground">
          <time dateTime={frontmatter.date}>
            {new Date(frontmatter.date).toLocaleDateString("en-US", {
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </time>
          {frontmatter.readingTime && <span>{frontmatter.readingTime} min read</span>}
          {frontmatter.tags && frontmatter.tags.length > 0 && (
            <>
              <span aria-hidden="true">&middot;</span>
              <div className="flex flex-wrap gap-2">
                {frontmatter.tags.map((tag: string) => (
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
      <div className="prose prose-neutral dark:prose-invert max-w-none">
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
