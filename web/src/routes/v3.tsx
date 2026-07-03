import { createFileRoute } from "@tanstack/react-router";
import { LandingV3 } from "#/components/landing/v3";

export const Route = createFileRoute("/v3")({
  head: () => ({
    meta: [
      { title: "gpui-query — Landing preview V3 · Blueprint" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV3,
});
