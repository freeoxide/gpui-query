import { createFileRoute } from "@tanstack/react-router";
import { LandingV4 } from "#/components/landing/v4";

export const Route = createFileRoute("/v4")({
  head: () => ({
    meta: [
      { title: "gpui-query - Landing preview V4 · Combined" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV4,
});
