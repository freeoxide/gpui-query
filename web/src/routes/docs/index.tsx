import { createFileRoute, Link } from "@tanstack/react-router";
import { getAllDocs } from "#/lib/content";
import { Card, CardHeader, CardTitle, CardDescription } from "#/components/ui/card";
import { ArrowRight } from "lucide-react";

export const Route = createFileRoute("/docs/")({
  loader: () => getAllDocs(),
  head: () => ({
    meta: [
      { title: "Documentation - gpui-query" },
      {
        name: "description",
        content:
          "Complete documentation for gpui-query, async state management for GPUI applications",
      },
      { property: "og:title", content: "Documentation - gpui-query" },
      {
        property: "og:description",
        content:
          "Complete documentation for gpui-query, async state management for GPUI applications",
      },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
      { name: "twitter:title", content: "Documentation - gpui-query" },
      {
        name: "twitter:description",
        content:
          "Complete documentation for gpui-query, async state management for GPUI applications",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/docs" }],
  }),
  component: DocsIndex,
});

function DocsIndex() {
  const docs = Route.useLoaderData();

  const grouped = docs.reduce<Record<string, typeof docs>>((acc, doc) => {
    const category = doc.frontmatter.category ?? "Documentation";
    if (!acc[category]) acc[category] = [];
    acc[category].push(doc);
    return acc;
  }, {});

  return (
    <div>
      <div className="mb-8 rounded-xl border border-primary/20 bg-gradient-to-r from-primary/10 via-primary/5 to-transparent p-6">
        <h1 className="text-2xl font-bold">Welcome to gpui-query Docs</h1>
        <p className="mt-2 text-muted-foreground">
          Everything you need to build reactive async UIs with GPUI
        </p>
      </div>

      {Object.entries(grouped).map(([category, categoryDocs]) => (
        <section key={category} className="mb-10">
          <h2 className="mb-4 border-l-4 border-primary pl-3 text-xl font-semibold text-foreground">
            {category}
          </h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {categoryDocs.map((doc) => (
              <Link key={doc.slug} to="/docs/$slug" params={{ slug: doc.slug }} className="group">
                <Card className="h-full transition-all duration-200 hover:-translate-y-0.5 hover:shadow-md hover:border-primary/50">
                  <CardHeader className="flex flex-row items-start justify-between gap-2">
                    <div className="flex-1">
                      <CardTitle className="text-base group-hover:text-primary">
                        {doc.frontmatter.title}
                      </CardTitle>
                      <CardDescription className="mt-1">
                        {doc.frontmatter.description}
                      </CardDescription>
                    </div>
                    <ArrowRight className="mt-1 h-4 w-4 shrink-0 text-muted-foreground opacity-0 transition-all duration-200 group-hover:translate-x-0.5 group-hover:text-primary group-hover:opacity-100" />
                  </CardHeader>
                </Card>
              </Link>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
