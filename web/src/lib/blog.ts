import { type ComponentType } from "react";

import IntroducingGpuiQueryPost from "../content/blog/introducing-gpui-query";
import WhyGpuiQueryPost from "../content/blog/why-gpui-query";
import CooperativeCancellationPost from "../content/blog/cooperative-cancellation";
import CachePoliciesExplainedPost from "../content/blog/cache-policies-explained";

/* ─── Frontmatter schema ────────────────────────────────────────────── */

export interface BlogFrontmatter {
  title: string;
  description: string;
  /** ISO date, YYYY-MM-DD */
  date: string;
  author: string;
  tags?: string[];
}

export interface BlogPost {
  slug: string;
  Content: ComponentType;
  frontmatter: BlogFrontmatter;
}

/* ─── Post registry ─────────────────────────────────────────────────── */

/**
 * Static post registry. Blog bodies are authored as TSX components (see
 * `src/content/blog/*.tsx`) so they prerender cleanly under TanStack Start's
 * SSR environment. MDX + `import.meta.glob` did not resolve module exports
 * during prerender, leaving posts empty — TSX components do not have that
 * problem. Frontmatter lives here alongside the component import.
 */
const POSTS: BlogPost[] = [
  {
    slug: "introducing-gpui-query",
    Content: IntroducingGpuiQueryPost,
    frontmatter: {
      title: "Introducing gpui-query",
      description:
        "Zero-boilerplate async state management for GPUI — inspired by TanStack Query, built for Rust.",
      date: "2026-03-10",
      author: "hmziqrs",
      tags: ["gpui", "rust", "async", "introduction"],
    },
  },
  {
    slug: "why-gpui-query",
    Content: WhyGpuiQueryPost,
    frontmatter: {
      title: "Why gpui-query",
      description:
        "Bringing TanStack Query's battle-tested async state patterns to GPUI and Rust — without the boilerplate.",
      date: "2026-03-12",
      author: "hmziqrs",
      tags: ["gpui", "rust", "async", "introduction"],
    },
  },
  {
    slug: "cooperative-cancellation",
    Content: CooperativeCancellationPost,
    frontmatter: {
      title: "Cooperative Cancellation in gpui-query",
      description:
        "How QuerySignal uses Arc<AtomicBool> to cancel in-flight queries cleanly when components unmount.",
      date: "2026-04-08",
      author: "hmziqrs",
      tags: ["gpui", "rust", "concurrency", "cancellation"],
    },
  },
  {
    slug: "cache-policies-explained",
    Content: CachePoliciesExplainedPost,
    frontmatter: {
      title: "Cache Policies, Explained",
      description:
        "NoCache vs Ttl vs StaleWhileRevalidate — when each CachePolicy variant is the right choice, and how they interact with retries and observers.",
      date: "2026-05-02",
      author: "hmziqrs",
      tags: ["gpui", "rust", "caching", "performance"],
    },
  },
];

/** All blog posts, sorted by date descending. */
export function getAllBlogPosts(): BlogPost[] {
  return [...POSTS].sort(
    (a, b) => new Date(b.frontmatter.date).getTime() - new Date(a.frontmatter.date).getTime(),
  );
}

/** Look up a single post by its slug (filename without extension). */
export function getBlogBySlug(slug: string): BlogPost | undefined {
  return POSTS.find((post) => post.slug === slug);
}

/** Slugs only — handy for prerendering and feeds. */
export function getBlogSlugs(): string[] {
  return POSTS.map((post) => post.slug);
}
