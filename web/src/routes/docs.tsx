import { useState, useCallback } from "react";
import { createFileRoute, Outlet } from "@tanstack/react-router";
import { DocsSidebar } from "#/components/docs-sidebar";
import { getAllDocs } from "#/lib/content";

export const Route = createFileRoute("/docs")({
  component: DocsLayout,
});

function MenuIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="4" x2="20" y1="12" y2="12" />
      <line x1="4" x2="20" y1="6" y2="6" />
      <line x1="4" x2="20" y1="18" y2="18" />
    </svg>
  );
}

function DocsLayout() {
  const docs = getAllDocs();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const sidebarItems = docs.map((doc) => ({
    slug: doc.slug,
    title: doc.frontmatter.title,
    category: doc.frontmatter.category,
  }));

  const closeSidebar = useCallback(() => setSidebarOpen(false), []);

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="flex gap-8">
        <DocsSidebar items={sidebarItems} mobileOpen={sidebarOpen} onMobileClose={closeSidebar} />
        <div className="min-w-0 flex-1">
          <Outlet />
        </div>
      </div>

      {/* Mobile sidebar trigger */}
      <button
        onClick={() => setSidebarOpen((prev) => !prev)}
        className="fixed bottom-6 left-6 z-30 flex h-12 w-12 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg md:hidden"
        aria-label="Toggle sidebar"
      >
        <MenuIcon />
      </button>
    </div>
  );
}
