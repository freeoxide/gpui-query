import { createFileRoute } from "@tanstack/react-router";
import { LandingV2 } from "#/components/landing/v2";

export const Route = createFileRoute("/v2")({
  head: () => ({
    meta: [
      { title: "gpui-query — Landing preview V2 · Control Room" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV2,
});
