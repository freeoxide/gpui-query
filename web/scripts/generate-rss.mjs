#!/usr/bin/env node
/**
 * Generate RSS 2.0 feed (rss.xml) from blog MDX frontmatter.
 *
 * Usage: node scripts/generate-rss.mjs [--output dist/client]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

const contentDir = resolve("src/content/blog");
const siteUrl = "https://gpui-query.hmziq.xyz";

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
    const frontmatter = {};
    if (fmMatch) {
      for (const line of fmMatch[1].split("\n")) {
        const [key, ...rest] = line.split(":");
        if (key && rest.length) {
          frontmatter[key.trim()] = rest
            .join(":")
            .trim()
            .replace(/^["']|["']$/g, "");
        }
      }
    }

    posts.push({ slug, frontmatter });
  }

  // Sort by date descending
  posts.sort((a, b) => new Date(b.frontmatter.date) - new Date(a.frontmatter.date));

  const items = posts
    .map(
      (post) => `    <item>
      <title><![CDATA[${post.frontmatter.title}]]></title>
      <link>${siteUrl}/blog/${post.slug}</link>
      <guid isPermaLink="true">${siteUrl}/blog/${post.slug}</guid>
      <description><![CDATA[${post.frontmatter.description}]]></description>
      <pubDate>${new Date(post.frontmatter.date).toUTCString()}</pubDate>
    </item>`,
    )
    .join("\n");

  const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>gpui-query Blog</title>
    <link>${siteUrl}/blog</link>
    <description>Async state management for GPUI</description>
    <language>en-us</language>
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
