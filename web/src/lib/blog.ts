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

/* ─── Post registry ─────────────────────────────────────────────────── */

interface BlogModule {
  default: ComponentType;
  frontmatter?: Partial<BlogFrontmatter>;
}

/**
 * Posts are MDX documents in `src/content/blog/*.mdx`. YAML frontmatter is
 * exposed as each module's `frontmatter` export by remark-mdx-frontmatter
 * (see vite.config.ts). The glob MUST stay `eager` — an eager glob compiles
 * to static imports, which resolve during TanStack Start's prerender exactly
 * like hand-written imports. A lazy glob leaves posts empty at prerender
 * time, which is the failure that originally pushed the blog to TSX bodies.
 */
const modules = import.meta.glob<BlogModule>("../content/blog/*.mdx", { eager: true });

const POSTS: BlogPost[] = Object.entries(modules).map(([path, mod]) => {
  const slug = path
    .split("/")
    .pop()!
    .replace(/\.mdx$/, "");
  const fm = mod.frontmatter;
  if (!fm?.title || !fm.description || !fm.date || !fm.author) {
    throw new Error(
      `Blog post "${slug}" is missing required frontmatter (title, description, date, author)`,
    );
  }
  return {
    slug,
    Content: mod.default,
    frontmatter: {
      title: fm.title,
      description: fm.description,
      date: fm.date,
      author: fm.author,
      tags: fm.tags,
    },
  };
});

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
