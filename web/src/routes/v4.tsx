import { createFileRoute, notFound } from "@tanstack/react-router";
import { LANDING_PREVIEWS_ENABLED } from "#/lib/flags";
import { LandingV4 } from "#/components/landing/v4";
import { softwareSourceCode } from "#/lib/seo";

const title = "gpui-query - Landing preview V4 · Combined";
const description =
  "A combined gpui-query landing preview with the live query schematic, QueryClient cache deck, hook API examples, code comparison, and annotated Rust lines.";
const url = "https://gpui-query.freeoxide.com/v4";
const image = "https://gpui-query.freeoxide.com/og-image.png";

export const Route = createFileRoute("/v4")({
  loader: () => {
    if (!LANDING_PREVIEWS_ENABLED) throw notFound();
  },
  head: () => ({
    meta: [
      { title },
      { name: "description", content: description },
      { name: "robots", content: "noindex" },
      { property: "og:title", content: title },
      { property: "og:description", content: description },
      { property: "og:type", content: "website" },
      { property: "og:url", content: url },
      { property: "og:image", content: image },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: title },
      { name: "twitter:description", content: description },
      { name: "twitter:image", content: image },
    ],
    links: [{ rel: "canonical", href: url }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify(
          softwareSourceCode({
            name: "gpui-query",
            description:
              "Async state management for GPUI with caching, retry, revalidation, and cooperative cancellation",
            programmingLanguage: "Rust",
            codeRepository: "https://github.com/freeoxide/gpui-query",
            url,
            license: "MIT",
          }),
        ),
      },
    ],
  }),
  component: LandingV4,
});
