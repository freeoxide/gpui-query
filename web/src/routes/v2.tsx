import { createFileRoute, notFound } from "@tanstack/react-router";
import { LANDING_PREVIEWS_ENABLED } from "#/lib/flags";
import { LandingV2 } from "#/components/landing/v2";

export const Route = createFileRoute("/v2")({
  loader: () => {
    if (!LANDING_PREVIEWS_ENABLED) throw notFound();
  },
  head: () => ({
    meta: [
      { title: "gpui-query — Landing preview V2 · Control Room" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV2,
});
