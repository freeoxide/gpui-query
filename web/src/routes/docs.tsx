import { createFileRoute, Outlet } from "@tanstack/react-router";
import { DocsSidebar } from "#/components/docs-sidebar";
import { getAllDocs } from "#/lib/content";

export const Route = createFileRoute("/docs")({
  component: DocsLayout,
});

function DocsLayout() {
  const docs = getAllDocs();
  const sidebarItems = docs.map((doc) => ({
    slug: doc.slug,
    title: doc.frontmatter.title,
    category: doc.frontmatter.category,
  }));

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="flex gap-8">
        <DocsSidebar items={sidebarItems} />
        <div className="min-w-0 flex-1">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
