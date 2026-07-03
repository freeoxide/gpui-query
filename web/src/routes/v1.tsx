import { createFileRoute } from "@tanstack/react-router";
import { LandingV1 } from "#/components/landing/v1";

export const Route = createFileRoute("/v1")({
  head: () => ({
    meta: [
      { title: "gpui-query — Landing preview V1 · Signal Path" },
      { name: "robots", content: "noindex" },
    ],
  }),
  component: LandingV1,
});
