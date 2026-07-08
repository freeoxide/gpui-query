import { createFileRoute, notFound } from "@tanstack/react-router";
import { LANDING_PREVIEWS_ENABLED } from "#/lib/flags";
import { LandingV3 } from "#/components/landing/v3";

export const Route = createFileRoute("/v3")({
  loader: () => {
    if (!LANDING_PREVIEWS_ENABLED) throw notFound();
  },
  head: () => ({
    meta: [
      { title: "gpui-query — Landing preview V3 · Blueprint" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV3,
});
