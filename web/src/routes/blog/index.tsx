import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/blog/")({
  component: BlogIndex,
});

function BlogIndex() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-6 lg:px-8">
      <h1 className="text-3xl font-bold">Blog</h1>
      <p className="mt-4 text-muted-foreground">Blog listing coming soon.</p>
    </div>
  );
}
