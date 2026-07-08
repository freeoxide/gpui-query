import { createFileRoute, notFound } from "@tanstack/react-router";
import { LANDING_PREVIEWS_ENABLED } from "#/lib/flags";
import { LandingV1 } from "#/components/landing/v1";

export const Route = createFileRoute("/v1")({
  loader: () => {
    if (!LANDING_PREVIEWS_ENABLED) throw notFound();
  },
  head: () => ({
    meta: [
      { title: "gpui-query — Landing preview V1 · Signal Path" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV1,
});
