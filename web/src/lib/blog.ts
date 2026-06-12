/**
 * Blog post utilities — load and parse MDX posts from src/content/blog/
 *
 * Each MDX file should have frontmatter with:
 *   title, date (YYYY-MM-DD), excerpt, author (optional)
 *
 * Example:
 *   ---
 *   title: "Hello World"
 *   date: "2024-06-12"
 *   excerpt: "First post"
 *   ---
 */

export interface BlogPostMeta {
  slug: string;
  title: string;
  date: string;
  excerpt: string;
  author?: string;
}

export interface BlogPost extends BlogPostMeta {
  Content: React.ComponentType;
}

/**
 * Import all MDX files from the blog content directory.
 * remarkMdxFrontmatter exposes frontmatter fields as named exports.
 */
const modules = import.meta.glob<{
  default: React.ComponentType;
  title?: string;
  date?: string;
  excerpt?: string;
  author?: string;
}>("/src/content/blog/*.mdx", { eager: true });

function slugFromPath(path: string): string {
  const filename = path.split("/").pop() ?? "";
  return filename.replace(/\.mdx$/, "");
}

/** Return metadata for all posts, sorted newest-first. */
export function getAllPosts(): BlogPostMeta[] {
  const posts: BlogPostMeta[] = [];

  for (const [path, mod] of Object.entries(modules)) {
    if (!mod.title || !mod.date) continue;
    posts.push({
      slug: slugFromPath(path),
      title: mod.title,
      date: mod.date,
      excerpt: mod.excerpt ?? "",
      author: mod.author,
    });
  }

  return posts.sort((a, b) => (a.date > b.date ? -1 : 1));
}

/** Return a single post by slug (including the rendered Content component). */
export function getPost(slug: string): BlogPost | null {
  for (const [path, mod] of Object.entries(modules)) {
    if (!mod.title || !mod.date) continue;
    if (slugFromPath(path) !== slug) continue;
    return {
      slug,
      title: mod.title,
      date: mod.date,
      excerpt: mod.excerpt ?? "",
      author: mod.author,
      Content: mod.default,
    };
  }
  return null;
}
