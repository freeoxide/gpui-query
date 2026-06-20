#!/usr/bin/env node
/**
 * Generate RSS 2.0 feed (rss.xml) from blog MDX frontmatter.
 *
 * This build-time script mirrors the runtime feed in src/lib/rss.ts (which
 * uses getAllBlogPosts() from src/lib/blog.ts). It reads the MDX files
 * directly because it runs outside the Vite/MDX bundler context.
 *
 * Usage: node scripts/generate-rss.mjs [--output .output/public]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

const contentDir = resolve("src/content/blog");
const siteUrl = "https://gpui-query.hmziq.xyz";

/** Parse a YAML frontmatter block into a flat object (sufficient for blog FM). */
function parseFrontmatter(block) {
  const fm = {};
  for (const line of block.split("\n")) {
    const m = line.match(/^([A-Za-z0-9_]+):\s*(.*)$/);
    if (!m) continue;
    const [, key, raw] = m;
    const value = raw.trim();
    // Inline array: ["a", "b", "c"]
    const arr = value.match(/^\[(.*)\]$/);
    if (arr) {
      fm[key] = arr[1]
        .split(",")
        .map((s) => s.trim().replace(/^["']|["']$/g, ""))
        .filter(Boolean);
    } else {
      fm[key] = value.replace(/^["']|["']$/g, "");
    }
  }
  return fm;
}

async function main() {
  let files;
  try {
    files = await readdir(contentDir);
  } catch {
    console.log("No blog directory found, skipping RSS generation");
    return;
  }

  const mdxFiles = files.filter((f) => f.endsWith(".mdx"));
  const posts = [];

  for (const file of mdxFiles) {
    const content = await readFile(join(contentDir, file), "utf-8");
    const slug = file.replace(/\.mdx$/, "");

    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    const frontmatter = fmMatch ? parseFrontmatter(fmMatch[1]) : {};

    posts.push({ slug, frontmatter });
  }

  // Sort by date descending — same ordering as getAllBlogPosts() in blog.ts.
  posts.sort((a, b) => new Date(b.frontmatter.date) - new Date(a.frontmatter.date));

  const items = posts
    .map((post) => {
      const categories = (post.frontmatter.tags || [])
        .map((t) => `      <category>${t}</category>`)
        .join("\n");
      return `    <item>
      <title><![CDATA[${post.frontmatter.title}]]></title>
      <link>${siteUrl}/blog/${post.slug}</link>
      <guid isPermaLink="true">${siteUrl}/blog/${post.slug}</guid>
      <description><![CDATA[${post.frontmatter.description}]]></description>
      <pubDate>${new Date(post.frontmatter.date).toUTCString()}</pubDate>${
        categories ? `\n${categories}` : ""
      }
    </item>`;
    })
    .join("\n");

  const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>gpui-query Blog</title>
    <link>${siteUrl}/blog</link>
    <description>Async state management for GPUI</description>
    <language>en-us</language>
    <atom:link href="${siteUrl}/rss.xml" rel="self" type="application/rss+xml" />
    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
    <ttl>60</ttl>
${items}
  </channel>
</rss>`;

  await mkdir(outputDir, { recursive: true });
  await writeFile(join(outputDir, "rss.xml"), rss, "utf-8");
  console.log(`Generated rss.xml (${posts.length} posts)`);
}

main().catch((err) => {
  console.error("Failed to generate RSS:", err);
  process.exit(1);
});
