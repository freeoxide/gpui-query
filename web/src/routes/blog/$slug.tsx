import { createFileRoute, Link, notFound } from "@tanstack/react-router";
import { getPost } from "#/lib/blog";
import { ArrowLeft, CalendarIcon, UserIcon } from "lucide-react";
import { Button } from "#/components/ui/button";

export const Route = createFileRoute("/blog/$slug")({
  loader: ({ params }) => {
    const post = getPost(params.slug);
    if (!post) {
      throw notFound();
    }
    return post;
  },
  head: ({ params }) => {
    const post = getPost(params.slug);
    const title = post ? `${post.title} - gpui-query` : "Post Not Found - gpui-query";
    const description = post?.excerpt ?? "Blog post from gpui-query.";
    return {
      meta: [
        { title },
        { name: "description", content: description },
        { property: "og:title", content: title },
        { property: "og:description", content: description },
        { property: "og:type", content: "article" },
        { property: "og:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
        { name: "twitter:card", content: "summary_large_image" },
        { name: "twitter:title", content: title },
        { name: "twitter:description", content: description },
        { name: "twitter:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
      ],
      links: [{ rel: "canonical", href: `https://gpui-query.hmziq.xyz/blog/${params.slug}` }],
    };
  },
  component: BlogPostPage,
});

function BlogPostPage() {
  const { slug } = Route.useParams();
  const post = getPost(slug);

  if (!post) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl text-center">
          <h1 className="text-3xl font-bold">Post not found</h1>
          <p className="mt-2 text-muted-foreground">
            The blog post you&apos;re looking for doesn&apos;t exist.
          </p>
          <Button asChild className="mt-6">
            <Link to="/blog">
              <ArrowLeft className="mr-1.5 h-4 w-4" />
              Back to Blog
            </Link>
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-16 lg:px-8">
      <article className="mx-auto max-w-3xl">
        {/* Back link */}
        <Link
          to="/blog"
          className="mb-8 inline-flex items-center gap-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-primary"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          Back to Blog
        </Link>

        {/* Header */}
        <header>
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">{post.title}</h1>
          <div className="mt-4 flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <CalendarIcon className="h-3.5 w-3.5" />
              <time dateTime={post.date}>
                {new Date(post.date).toLocaleDateString("en-US", {
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                })}
              </time>
            </span>
            {post.author && (
              <span className="inline-flex items-center gap-1.5">
                <UserIcon className="h-3.5 w-3.5" />
                {post.author}
              </span>
            )}
          </div>
        </header>

        {/* Divider */}
        <hr className="my-8 border-border/60" />

        {/* Content */}
        <div className="prose-container max-w-none [&_h2]:mt-10 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:tracking-tight [&_h3]:mt-8 [&_h3]:text-lg [&_h3]:font-semibold [&_p]:leading-relaxed [&_p]:text-foreground/90 [&_a]:text-primary [&_a]:underline [&_a]:decoration-primary/30 [&_a]:underline-offset-2 [&_a:hover]:decoration-primary [&_code]:rounded [&_code]:bg-muted [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-sm [&_pre]:rounded-lg [&_pre]:border [&_pre]:bg-muted/50 [&_pre]:p-4 [&_ul]:my-4 [&_ul]:ml-6 [&_ul]:list-disc [&_ul]:space-y-1 [&_li]:text-foreground/90 [&_li]:leading-relaxed [&_blockquote]:border-l-4 [&_blockquote]:border-primary/40 [&_blockquote]:pl-4 [&_blockquote]:italic [&_blockquote]:text-muted-foreground">
          <post.Content />
        </div>

        {/* Bottom nav */}
        <hr className="my-10 border-border/60" />
        <Button variant="outline" asChild>
          <Link to="/blog">
            <ArrowLeft className="mr-1.5 h-4 w-4" />
            Back to Blog
          </Link>
        </Button>
      </article>
    </div>
  );
}
