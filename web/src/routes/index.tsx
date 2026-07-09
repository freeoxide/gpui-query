import { createFileRoute } from "@tanstack/react-router";
import { LandingV4 } from "#/components/landing/v4";
import { softwareSourceCode } from "#/lib/seo";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "gpui-query — Async State Management for GPUI" },
      {
        name: "description",
        content:
          "Zero-boilerplate async state management for GPUI. Caching, retry, cooperative cancellation, and persistence for the Zed editor's framework.",
      },
      { property: "og:title", content: "gpui-query — Async State Management for GPUI" },
      {
        property: "og:description",
        content:
          "Zero-boilerplate async state management for GPUI. Caching, retry, cooperative cancellation, and persistence.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://gpui-query.freeoxide.com" },
      { property: "og:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
      { property: "og:image:alt", content: "gpui-query - Async State Management for GPUI" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "gpui-query — Async State Management for GPUI" },
      { name: "twitter:description", content: "Zero-boilerplate async state management for GPUI." },
      { name: "twitter:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.freeoxide.com" }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify(
          softwareSourceCode({
            name: "gpui-query",
            description: "Zero-boilerplate async state management for GPUI",
            programmingLanguage: "Rust",
            codeRepository: "https://github.com/freeoxide/gpui-query",
            url: "https://gpui-query.freeoxide.com",
            license: "MIT",
          }),
        ),
      },
    ],
  }),
  // V4 won the landing preview bake-off and now renders as the homepage.
  // The /v1–/v4 preview routes are gated behind LANDING_PREVIEWS_ENABLED.
  component: LandingV4,
});
