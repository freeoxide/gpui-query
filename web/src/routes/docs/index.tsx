import { createFileRoute, Link } from "@tanstack/react-router";
import { getAllDocs } from "#/lib/content";
import { Card, CardHeader, CardTitle, CardDescription } from "#/components/ui/card";

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
    ],
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
      <header className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Documentation</h1>
        <p className="mt-2 text-lg text-muted-foreground">
          Everything you need to know about using gpui-query in your GPUI applications.
        </p>
      </header>

      {Object.entries(grouped).map(([category, categoryDocs]) => (
        <section key={category} className="mb-10">
          <h2 className="mb-4 text-xl font-semibold text-foreground">{category}</h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {categoryDocs.map((doc) => (
              <Link
                key={doc.slug}
                to="/docs/$slug"
                params={{ slug: doc.slug }}
                className="group transition-transform hover:scale-[1.02]"
              >
                <Card className="h-full transition-colors group-hover:border-primary/50">
                  <CardHeader>
                    <CardTitle className="text-base group-hover:text-primary">
                      {doc.frontmatter.title}
                    </CardTitle>
                    <CardDescription>{doc.frontmatter.description}</CardDescription>
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
