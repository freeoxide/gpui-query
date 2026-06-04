import { createFileRoute, Link } from "@tanstack/react-router";
import { Github, ExternalLink } from "lucide-react";
import { Button } from "#/components/ui/button";

export const Route = createFileRoute("/about")({
  head: () => ({
    meta: [
      { title: "About - gpui-query" },
      {
        name: "description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
      { property: "og:title", content: "About - gpui-query" },
      {
        property: "og:description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
      { name: "twitter:title", content: "About - gpui-query" },
      {
        name: "twitter:description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/about" }],
  }),
  component: AboutPage,
});

function AboutPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-3xl">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">About gpui-query</h1>

        <div className="mt-8 space-y-6 leading-relaxed text-muted-foreground">
          <p>
            <strong className="text-foreground">gpui-query</strong> brings the reactive query
            patterns you love from the web world into Rust GPUI applications. Fetch, cache, and
            synchronize async data with a single hook — no manual lifecycle management required.
          </p>

          <p>
            Built on a layered architecture with a framework-agnostic Core, a Client layer for
            caching and garbage collection, and a Hook layer that integrates directly with GPUI's
            reactive primitives, gpui-query is designed to feel natural in the Rust ecosystem while
            providing battle-tested patterns from TanStack Query.
          </p>

          <h2 className="text-xl font-semibold text-foreground">Author</h2>
          <p>
            Created and maintained by <strong className="text-foreground">hmziq</strong>.
          </p>

          <h2 className="text-xl font-semibold text-foreground">Links</h2>
          <div className="flex flex-wrap gap-3">
            <Button variant="outline" size="sm" asChild>
              <a
                href="https://github.com/hmziqrs/gpui-query"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="mr-1 h-4 w-4" />
                GitHub
                <ExternalLink className="ml-1 h-3 w-3" />
              </a>
            </Button>
            <Button variant="outline" size="sm" asChild>
              <a href="https://zed.dev" target="_blank" rel="noopener noreferrer">
                Zed Editor
                <ExternalLink className="ml-1 h-3 w-3" />
              </a>
            </Button>
            <Button variant="outline" size="sm" asChild>
              <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
                Documentation
              </Link>
            </Button>
          </div>

          <h2 className="text-xl font-semibold text-foreground">License</h2>
          <p>
            gpui-query is open-source software released under the{" "}
            <strong className="text-foreground">MIT License</strong>. You are free to use, modify,
            and distribute it in your projects.
          </p>
        </div>
      </div>
    </div>
  );
}
