import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/changelog")({
  component: ChangelogPage,
});

function ChangelogPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <h1 className="text-3xl font-bold">Changelog</h1>
      <p className="mt-4 text-muted-foreground">Changelog coming soon.</p>
    </div>
  );
}
