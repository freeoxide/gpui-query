/* ─── Frontmatter schema ────────────────────────────────────────────── */

export interface BlogFrontmatter {
  title: string;
  description: string;
  /** ISO date, YYYY-MM-DD */
  date: string;
  author: string;
  tags?: string[];
}

export interface BlogPostMeta {
  slug: string;
  frontmatter: BlogFrontmatter;
}

/* ─── Post registry (metadata only) ─────────────────────────────────── */

/**
 * Posts are MDX documents in `src/content/blog/*.mdx`. This module is
 * reachable from route loaders/head, which TanStack keeps in the non-split
 * route tree — i.e. the entry chunk served to every page. So it must only
 * import frontmatter: the `?frontmatter-only` query is handled by the
 * mdx-frontmatter-only plugin in vite.config.ts, which emits just the parsed
 * YAML frontmatter and never compiles the post body. The compiled MDX
 * components live in `blog-content.ts`, imported only from the code-split
 * route components.
 *
 * The glob MUST stay `eager` — an eager glob compiles to static imports,
 * which resolve during TanStack Start's prerender exactly like hand-written
 * imports. A lazy glob leaves posts empty at prerender time, which is the
 * failure that originally pushed the blog to TSX bodies.
 */
const modules = import.meta.glob<Partial<BlogFrontmatter> | undefined>(
  "../content/blog/*.mdx",
  { eager: true, import: "frontmatter", query: "?frontmatter-only" },
);

const POSTS: BlogPostMeta[] = Object.entries(modules).map(([path, fm]) => {
  const slug = path
    .split("/")
    .pop()!
    .replace(/\.mdx$/, "");
  if (!fm?.title || !fm.description || !fm.date || !fm.author) {
    throw new Error(
      `Blog post "${slug}" is missing required frontmatter (title, description, date, author)`,
    );
  }
  return {
    slug,
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
export function getAllBlogPosts(): BlogPostMeta[] {
  return [...POSTS].sort(
    (a, b) => new Date(b.frontmatter.date).getTime() - new Date(a.frontmatter.date).getTime(),
  );
}

/** Look up a single post by its slug (filename without extension). */
export function getBlogBySlug(slug: string): BlogPostMeta | undefined {
  return POSTS.find((post) => post.slug === slug);
}

/** Slugs only — handy for prerendering and feeds. */
export function getBlogSlugs(): string[] {
  return POSTS.map((post) => post.slug);
}
