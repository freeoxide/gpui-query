import { type ComponentType } from "react";

import IntroducingGpuiQueryPost from "../content/blog/introducing-gpui-query";

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
