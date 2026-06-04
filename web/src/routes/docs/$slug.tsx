import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/docs/$slug")({
  component: DocPage,
});

function DocPage() {
  const { slug } = Route.useParams();
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <h1 className="text-3xl font-bold">{slug}</h1>
      <p className="mt-4 text-muted-foreground">Doc page coming soon.</p>
    </div>
  );
}
