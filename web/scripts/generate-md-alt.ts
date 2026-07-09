#!/usr/bin/env node
/**
 * Generate clean .md alternatives for Docusaurus documentation pages.
 * AI crawlers can fetch these directly for plain markdown content.
 *
 * Source: Docusaurus docs at ../docs/docs (all .md and .mdx files).
 * Output: {output}/docs/{route}.md
 *
 * Usage: node scripts/generate-md-alt.ts [--output .output/public]
 */

import { writeFile, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import process from "node:process";
import { loadDocs } from "./lib/docs-md.ts";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

// Scripts run from web/, so the Docusaurus docs live one level up.
const docsRoot = resolve("..", "docs", "docs");

async function main(): Promise<void> {
  const docs = await loadDocs(docsRoot);
  if (docs.length === 0) {
    console.log("No Docusaurus docs found, skipping .md generation");
    return;
  }

  const docsOutDir = join(outputDir, "docs");
  await mkdir(docsOutDir, { recursive: true });

  for (const doc of docs) {
    // route may contain subdirectory segments (e.g. "guides/caching").
    const outFile = join(docsOutDir, `${doc.route || "index"}.md`);
    await mkdir(dirname(outFile), { recursive: true });
    await writeFile(outFile, `${doc.plain}\n`, "utf-8");
  }

  console.log(`Generated ${docs.length} .md alternative files in ${docsOutDir}`);
}

main().catch((err: unknown) => {
  console.error("Failed to generate .md alternatives:", err);
  process.exit(1);
});
