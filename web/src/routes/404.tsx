import { createFileRoute, Link } from "@tanstack/react-router";
import { Home, BookOpen } from "lucide-react";
import { Button } from "#/components/ui/button";

export const Route = createFileRoute("/404")({
  head: () => ({
    meta: [
      { title: "Page Not Found - gpui-query" },
      {
        name: "description",
        content: "The page you are looking for does not exist.",
      },
    ],
  }),
  component: NotFoundPage,
});

function NotFoundPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="flex min-h-[60vh] flex-col items-center justify-center text-center">
        <h1 className="text-8xl font-extrabold tracking-tighter text-primary">404</h1>
        <p className="mt-4 text-2xl font-semibold text-foreground">Page not found</p>
        <p className="mt-2 max-w-md text-muted-foreground">
          The page you are looking for does not exist or has been moved.
        </p>
        <div className="mt-8 flex flex-col items-center gap-3 sm:flex-row">
          <Button asChild>
            <Link to="/">
              <Home className="mr-1 h-4 w-4" />
              Go Home
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
              <BookOpen className="mr-1 h-4 w-4" />
              Documentation
            </Link>
          </Button>
        </div>
      </div>
    </div>
  );
}
