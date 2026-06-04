import { createFileRoute, Link } from "@tanstack/react-router";
import { getDocBySlug, getAllDocs } from "#/lib/content";
import { DocsPagination } from "#/components/docs-pagination";
import { techArticle } from "#/lib/seo";
import { Button } from "#/components/ui/button";
import { Home, BookOpen } from "lucide-react";
import { useMemo } from "react";

export const Route = createFileRoute("/docs/$slug")({
  loader: ({ params: { slug } }) => {
    const doc = getDocBySlug(slug);
    if (!doc) {
      throw new Error(`Doc not found: ${slug}`);
    }
    return doc;
  },
  head: ({ loaderData }) => {
    if (!loaderData) return { meta: [], links: [], scripts: [] };
    const fm = loaderData.frontmatter;
    const slug = fm.title.toLowerCase().replace(/\s+/g, "-");
    return {
      meta: [
        { title: `${fm.title} - gpui-query Docs` },
        { name: "description", content: fm.description },
        { property: "og:title", content: fm.title },
        { property: "og:description", content: fm.description },
        { property: "og:type", content: "article" },
        { name: "twitter:card", content: "summary" },
        { name: "twitter:title", content: fm.title },
        { name: "twitter:description", content: fm.description },
      ],
      links: [
        {
          rel: "canonical",
          href: `https://gpui-query.hmziq.xyz/docs/${slug}`,
        },
      ],
      scripts: [
        {
          type: "application/ld+json",
          children: JSON.stringify(
            techArticle({
              headline: fm.title,
              description: fm.description,
              url: `https://gpui-query.hmziq.xyz/docs/${slug}`,
            }),
          ),
        },
      ],
    };
  },
  component: DocPage,
  notFoundComponent: DocNotFound,
});

function DocPage() {
  const doc = Route.useLoaderData();
  const Content = doc.Content;

  const allDocs = getAllDocs();
  const currentIndex = allDocs.findIndex((d) => d.slug === Route.useParams().slug);

  const prev = useMemo(() => {
    if (currentIndex > 0) {
      const d = allDocs[currentIndex - 1];
      return { slug: d.slug, title: d.frontmatter.title };
    }
    return undefined;
  }, [allDocs, currentIndex]);

  const next = useMemo(() => {
    if (currentIndex < allDocs.length - 1) {
      const d = allDocs[currentIndex + 1];
      return { slug: d.slug, title: d.frontmatter.title };
    }
    return undefined;
  }, [allDocs, currentIndex]);

  return (
    <article>
      <header className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">{doc.frontmatter.title}</h1>
        <p className="mt-2 text-lg text-muted-foreground">{doc.frontmatter.description}</p>
      </header>
      <div className="prose prose-neutral dark:prose-invert max-w-none">
        <Content />
      </div>
      <DocsPagination prev={prev} next={next} />
    </article>
  );
}

function DocNotFound() {
  return (
    <div className="flex min-h-[40vh] flex-col items-center justify-center text-center">
      <h1 className="text-6xl font-extrabold tracking-tighter text-primary">404</h1>
      <p className="mt-4 text-xl font-semibold text-foreground">Documentation page not found</p>
      <p className="mt-2 max-w-md text-muted-foreground">
        The documentation page you are looking for does not exist or has been moved.
      </p>
      <div className="mt-8 flex flex-col items-center gap-3 sm:flex-row">
        <Button asChild>
          <Link to="/">
            <Home className="mr-1 h-4 w-4" />
            Go Home
          </Link>
        </Button>
        <Button variant="outline" asChild>
          <Link to="/docs">
            <BookOpen className="mr-1 h-4 w-4" />
            All Documentation
          </Link>
        </Button>
      </div>
    </div>
  );
}
