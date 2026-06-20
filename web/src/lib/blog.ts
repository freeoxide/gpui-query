import { type ComponentType } from "react";

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

/* ─── Loader ────────────────────────────────────────────────────────── */

/**
 * Eagerly import every blog MDX file. The MDX plugin
 * (remark-mdx-frontmatter) exposes frontmatter keys as named exports,
 * so each module carries `default` (the React component) plus the
 * frontmatter fields directly on the module object.
 */
const modules = import.meta.glob("../content/blog/*.mdx", { eager: true }) as Record<
  string,
  BlogFrontmatter & { default: ComponentType }
>;

function toSlug(path: string): string {
  return path
    .split("/")
    .pop()!
    .replace(/\.mdx$/, "");
}

function toPost(key: string, mod: BlogFrontmatter & { default: ComponentType }): BlogPost {
  const { default: Content, ...frontmatter } = mod;
  return {
    slug: toSlug(key),
    Content,
    frontmatter: frontmatter as BlogFrontmatter,
  };
}

/** All blog posts, sorted by date descending. */
export function getAllBlogPosts(): BlogPost[] {
  return Object.entries(modules)
    .map(([key, mod]) => toPost(key, mod))
    .sort(
      (a, b) => new Date(b.frontmatter.date).getTime() - new Date(a.frontmatter.date).getTime(),
    );
}

/** Look up a single post by its slug (filename without extension). */
export function getBlogBySlug(slug: string): BlogPost | undefined {
  return getAllBlogPosts().find((post) => post.slug === slug);
}

/** Slugs only — handy for prerendering and feeds. */
export function getBlogSlugs(): string[] {
  return getAllBlogPosts().map((post) => post.slug);
}
