import { createFileRoute } from "@tanstack/react-router";
import {
  HeroSection,
  FeatureShowcase,
  QuickStart,
  ComparisonSection,
  ArchitectureSection,
  CtaSection,
} from "#/components/landing";
import { useScrollReveal } from "#/hooks/use-scroll-reveal";
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
      { property: "og:url", content: "https://gpui-query.hmziq.xyz" },
      { property: "og:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "gpui-query — Async State Management for GPUI" },
      { name: "twitter:description", content: "Zero-boilerplate async state management for GPUI." },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz" }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify(
          softwareSourceCode({
            name: "gpui-query",
            description: "Zero-boilerplate async state management for GPUI",
            programmingLanguage: "Rust",
            codeRepository: "https://github.com/hmziqrs/gpui-query",
            url: "https://gpui-query.hmziq.xyz",
            license: "MIT",
          }),
        ),
      },
    ],
  }),
  component: Home,
});

function Home() {
  const revealRef = useScrollReveal();
  return (
    <div ref={revealRef} className="flex flex-col">
      <HeroSection />
      <FeatureShowcase />
      <QuickStart />
      <ComparisonSection />
      <ArchitectureSection />
      <CtaSection />
    </div>
  );
}
