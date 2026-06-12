#!/usr/bin/env node
/**
 * Generate .md alternatives for documentation pages.
 * AI crawlers can fetch these directly for clean markdown content.
 *
 * Reads from the Docusaurus source at ../../website/docs/ and mirrors
 * the directory structure into the output/docs/ directory.
 *
 * Usage: node scripts/generate-md-alt.mjs [--output dist/client]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, resolve, dirname } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

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

async function main() {
  const files = await collectMdxFiles(docusaurusDocsDir);

  if (files.length === 0) {
    console.log("No docs found in Docusaurus source, skipping .md generation");
    return;
  }

  let generated = 0;

  for (const filePath of files) {
    const content = await readFile(filePath, "utf-8");
    const relativePath = filePath.slice(docusaurusDocsDir.length + 1).replace(/\.mdx$/, ".md");

    const outPath = join(outputDir, "docs", relativePath);
    await mkdir(dirname(outPath), { recursive: true });

    // Strip frontmatter, convert JSX to plain text
    const body = content
      .replace(/^---\n[\s\S]*?\n---\n*/, "")
      .replace(/<Callout[^>]*>/g, "> ")
      .replace(/<\/Callout>/g, "")
      .replace(/<[^>]+>/g, "")
      .trim();

    await writeFile(outPath, body, "utf-8");
    generated++;
  }

  console.log(`Generated ${generated} .md alternative files`);
}

main().catch((err) => {
  console.error("Failed to generate .md alternatives:", err);
  process.exit(1);
});
