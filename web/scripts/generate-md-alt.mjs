#!/usr/bin/env node
/**
 * Generate .md alternatives for documentation pages.
 * AI crawlers can fetch these directly for clean markdown content.
 *
 * Usage: node scripts/generate-md-alt.mjs [--output .output/public]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

const contentDir = resolve("src/content/docs");

async function main() {
  const docsDir = join(outputDir, "docs");
  await mkdir(docsDir, { recursive: true });

  let files;
  try {
    files = await readdir(contentDir);
  } catch {
    console.log("No docs directory found, skipping .md generation");
    return;
  }

  const mdxFiles = files.filter((f) => f.endsWith(".mdx"));
  let generated = 0;

  for (const file of mdxFiles) {
    const content = await readFile(join(contentDir, file), "utf-8");
    const slug = file.replace(/\.mdx$/, "");

    // Strip frontmatter
    const body = content
      .replace(/^---\n[\s\S]*?\n---\n*/, "")
      // Convert JSX callouts to markdown blockquotes
      .replace(/<Callout[^>]*>/g, "> ")
      .replace(/<\/Callout>/g, "")
      // Remove remaining JSX tags
      .replace(/<[^>]+>/g, "")
      .trim();

    await writeFile(join(docsDir, `${slug}.md`), body, "utf-8");
    generated++;
  }

  console.log(`Generated ${generated} .md alternative files in ${docsDir}`);
}

main().catch((err) => {
  console.error("Failed to generate .md alternatives:", err);
  process.exit(1);
});
