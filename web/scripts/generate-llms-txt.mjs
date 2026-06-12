#!/usr/bin/env node
/**
 * Generate llms.txt and llms-full.txt for AI agent optimization.
 * Follows the llmstxt.org specification.
 *
 * Usage: node scripts/generate-llms-txt.mjs [--output .output/public]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

const contentDir = resolve("src/content");

async function readMdxFiles(subdir) {
  const dir = join(contentDir, subdir);
  let files;
  try {
    files = await readdir(dir);
  } catch {
    return [];
  }

  const mdxFiles = files.filter((f) => f.endsWith(".mdx"));
  const results = [];

  for (const file of mdxFiles) {
    const content = await readFile(join(dir, file), "utf-8");
    const slug = file.replace(/\.mdx$/, "");

    // Extract frontmatter
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

    // Strip frontmatter and JSX components for plain text
    const body = content
      .replace(/^---\n[\s\S]*?\n---\n*/, "") // Remove frontmatter
      .replace(/<Callout[^>]*>/g, "> ") // Convert callouts to blockquotes
      .replace(/<\/Callout>/g, "")
      .replace(/<[^>]+>/g, "") // Remove JSX tags
      .trim();

    results.push({ slug, frontmatter, body });
  }

  return results;
}

async function main() {
  await mkdir(outputDir, { recursive: true });

  const docs = await readMdxFiles("docs");
  const blogPosts = await readMdxFiles("blog");

  // Sort docs by order
  docs.sort((a, b) => (a.frontmatter.order || 0) - (b.frontmatter.order || 0));

  // Sort blog posts by date descending
  blogPosts.sort((a, b) => (b.frontmatter.date || "").localeCompare(a.frontmatter.date || ""));

  // Generate llms.txt (summary)
  const llmsLines = [
    "# gpui-query",
    "",
    "> Zero-boilerplate async state management for GPUI. Brings TanStack Query patterns to Rust and the Zed editor's GPUI framework with caching, retry, cooperative cancellation, and persistence.",
    "",
    "## Documentation",
    "",
  ];

  for (const doc of docs) {
    const url = `https://gpui-query.hmziq.xyz/docs/${doc.slug}`;
    llmsLines.push(`- [${doc.frontmatter.title}](${url}): ${doc.frontmatter.description}`);
  }

  if (blogPosts.length > 0) {
    llmsLines.push("");
    llmsLines.push("## Blog");
    llmsLines.push("");
    for (const post of blogPosts) {
      const url = `https://gpui-query.hmziq.xyz/blog/${post.slug}`;
      llmsLines.push(`- [${post.frontmatter.title}](${url}): ${post.frontmatter.description}`);
    }
  }

  llmsLines.push("");
  llmsLines.push("## Links");
  llmsLines.push("");
  llmsLines.push("- [GitHub](https://github.com/hmziqrs/gpui-query): Source code and issues");
  llmsLines.push(
    "- [Getting Started](https://gpui-query.hmziq.xyz/docs/getting-started): Install and first query",
  );

  const llmsTxt = llmsLines.join("\n");
  await writeFile(join(outputDir, "llms.txt"), llmsTxt, "utf-8");
  console.log(`Generated llms.txt (${llmsTxt.split("\n").length} lines)`);

  // Generate llms-full.txt (complete content)
  const fullLines = [
    "# gpui-query",
    "",
    "> Zero-boilerplate async state management for GPUI. Brings TanStack Query patterns to Rust and the Zed editor's GPUI framework.",
    "",
  ];

  for (const doc of docs) {
    fullLines.push(`## ${doc.frontmatter.title}`);
    fullLines.push("");
    fullLines.push(doc.body);
    fullLines.push("");
  }

  if (blogPosts.length > 0) {
    fullLines.push("---");
    fullLines.push("");
    for (const post of blogPosts) {
      fullLines.push(`## ${post.frontmatter.title}`);
      fullLines.push("");
      fullLines.push(`*Published: ${post.frontmatter.date}*`);
      fullLines.push("");
      fullLines.push(post.body);
      fullLines.push("");
    }
  }

  const llmsFullTxt = fullLines.join("\n");
  await writeFile(join(outputDir, "llms-full.txt"), llmsFullTxt, "utf-8");
  console.log(`Generated llms-full.txt (${llmsFullTxt.split("\n").length} lines)`);
}

main().catch((err) => {
  console.error("Failed to generate llms.txt:", err);
  process.exit(1);
});
