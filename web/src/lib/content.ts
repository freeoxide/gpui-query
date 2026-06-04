import type { ComponentType } from "react";

interface DocFrontmatter {
  title: string;
  description: string;
  category?: string;
  order?: number;
}

interface BlogFrontmatter {
  title: string;
  description: string;
  date: string;
  author?: string;
  tags?: string[];
  readingTime?: number;
}

interface MdxModule<T> {
  default: ComponentType;
  frontmatter: T;
}

const docModules = import.meta.glob<MdxModule<DocFrontmatter>>("../content/docs/*.mdx", {
  eager: true,
});

const blogModules = import.meta.glob<MdxModule<BlogFrontmatter>>("../content/blog/*.mdx", {
  eager: true,
});

function extractSlug(filePath: string): string {
  const fileName = filePath.split("/").pop() ?? "";
  return fileName.replace(/\.mdx$/, "");
}

export function getDocBySlug(slug: string): {
  Content: ComponentType;
  frontmatter: DocFrontmatter;
} | null {
  for (const [path, mod] of Object.entries(docModules)) {
    if (extractSlug(path) === slug) {
      return { Content: mod.default, frontmatter: mod.frontmatter };
    }
  }
  return null;
}

export function getAllDocs(): { slug: string; frontmatter: DocFrontmatter }[] {
  const docs = Object.entries(docModules).map(([path, mod]) => ({
    slug: extractSlug(path),
    frontmatter: mod.frontmatter,
  }));

  return docs.sort((a, b) => {
    const catA = a.frontmatter.category ?? "";
    const catB = b.frontmatter.category ?? "";
    if (catA !== catB) return catA.localeCompare(catB);
    return (a.frontmatter.order ?? 0) - (b.frontmatter.order ?? 0);
  });
}

export function getAllDocSlugs(): string[] {
  return getAllDocs().map((doc) => doc.slug);
}

export function getBlogBySlug(slug: string): {
  Content: ComponentType;
  frontmatter: BlogFrontmatter;
} | null {
  for (const [path, mod] of Object.entries(blogModules)) {
    if (extractSlug(path) === slug) {
      return { Content: mod.default, frontmatter: mod.frontmatter };
    }
  }
  return null;
}

export function getBlogPosts(): {
  slug: string;
  frontmatter: BlogFrontmatter;
}[] {
  const posts = Object.entries(blogModules).map(([path, mod]) => ({
    slug: extractSlug(path),
    frontmatter: mod.frontmatter,
  }));

  return posts.sort((a, b) => {
    return b.frontmatter.date.localeCompare(a.frontmatter.date);
  });
}

export function getAllBlogSlugs(): string[] {
  return getBlogPosts().map((post) => post.slug);
}

export type { DocFrontmatter, BlogFrontmatter };
