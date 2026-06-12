#!/usr/bin/env node
/**
 * Generate llms.txt and llms-full.txt for AI agent optimization.
 * Follows the llmstxt.org specification.
 *
 * Reads docs from the Docusaurus source at ../../website/docs/
 * and blog posts from src/content/blog/
 *
 * Usage: node scripts/generate-llms-txt.mjs [--output dist/client]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

const blogDir = resolve("src/content/blog");
const docusaurusDocsDir = resolve("../website/docs");

/**
 * Recursively collect all .mdx files from a directory tree.
 */
async function collectMdxFiles(dir) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return [];
  }

  const results = [];

  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      const nested = await collectMdxFiles(fullPath);
      results.push(...nested);
    } else if (entry.name.endsWith(".mdx")) {
      results.push(fullPath);
    }
  }

  return results;
}

function parseMdx(filePath, baseDir) {
  return async () => {
    const content = await readFile(filePath, "utf-8");
    // slug is the relative path from baseDir, without .mdx extension
    const slug = filePath.slice(baseDir.length + 1).replace(/\.mdx$/, "");

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

    // Strip frontmatter and JSX for plain text
    const body = content
      .replace(/^---\n[\s\S]*?\n---\n*/, "")
      .replace(/<Callout[^>]*>/g, "> ")
      .replace(/<\/Callout>/g, "")
      .replace(/<[^>]+>/g, "")
      .trim();

    return { slug, frontmatter, body };
  };
}

async function main() {
  await mkdir(outputDir, { recursive: true });

  // Collect docs from Docusaurus source
  const docFiles = await collectMdxFiles(docusaurusDocsDir);
  const docs = await Promise.all(docFiles.map((f) => parseMdx(f, docusaurusDocsDir)()));

  // Collect blog posts from TanStack Start source
  const blogFiles = await collectMdxFiles(blogDir);
  const blogPosts = await Promise.all(blogFiles.map((f) => parseMdx(f, blogDir)()));

  // Sort docs by slug for consistent ordering
  docs.sort((a, b) => a.slug.localeCompare(b.slug));

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
    const title = doc.frontmatter.title || doc.slug;
    const desc = doc.frontmatter.description || doc.frontmatter.excerpt || "";
    const url = `https://gpui-query.hmziq.xyz/docs/${doc.slug}`;
    llmsLines.push(`- [${title}](${url})${desc ? ": " + desc : ""}`);
  }

  if (blogPosts.length > 0) {
    llmsLines.push("");
    llmsLines.push("## Blog");
    llmsLines.push("");
    for (const post of blogPosts) {
      const title = post.frontmatter.title || post.slug;
      const desc = post.frontmatter.excerpt || post.frontmatter.description || "";
      const url = `https://gpui-query.hmziq.xyz/blog/${post.slug}`;
      llmsLines.push(`- [${title}](${url})${desc ? ": " + desc : ""}`);
    }
  }

  llmsLines.push("");
  llmsLines.push("## Links");
  llmsLines.push("");
  llmsLines.push("- [GitHub](https://github.com/hmziqrs/gpui-query): Source code and issues");
  llmsLines.push(
    "- [Getting Started](https://gpui-query.hmziq.xyz/docs/getting-started/installation): Install and first query",
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
    fullLines.push(`## ${doc.frontmatter.title || doc.slug}`);
    fullLines.push("");
    fullLines.push(doc.body);
    fullLines.push("");
  }

  if (blogPosts.length > 0) {
    fullLines.push("---");
    fullLines.push("");
    for (const post of blogPosts) {
      fullLines.push(`## ${post.frontmatter.title || post.slug}`);
      fullLines.push("");
      if (post.frontmatter.date) fullLines.push(`*Published: ${post.frontmatter.date}*`);
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
