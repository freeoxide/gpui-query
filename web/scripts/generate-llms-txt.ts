#!/usr/bin/env node
/**
 * Generate llms.txt and llms-full.txt for AI agent optimization.
 * Follows the llmstxt.org specification.
 *
 * Source: Starlight docs at src/content/docs/docs (all .md and .mdx files).
 *
 * Usage: node scripts/generate-llms-txt.ts [--output .output/public]
 */

import { writeFile, mkdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import process from "node:process";
import { loadDocs } from "./lib/docs-md.ts";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : "dist/client";

// Scripts run from web/. Starlight docs live under src/content/docs/docs and
// are served at /docs/** (route "" -> /docs/).
const docsRoot = resolve("src", "content", "docs", "docs");
const siteUrl = "https://gpui-query.freeoxide.com";

const HEADER = [
  "# gpui-query",
  "",
  "> Zero-boilerplate async state management for GPUI. Brings TanStack Query patterns to Rust and the Zed editor's GPUI framework with caching, retry, cooperative cancellation, and persistence.",
  "",
];

async function main(): Promise<void> {
  const parsed = await loadDocs(docsRoot);
  if (parsed.length === 0) {
    console.log("No docs found, skipping llms.txt generation");
    return;
  }

  const docs = parsed.map((doc) => ({
    route: doc.route,
    title: doc.frontmatter.title || doc.route || "Home",
    description: doc.frontmatter.description || "",
    order: Number(doc.frontmatter.sidebar_position ?? 0),
    plain: doc.plain,
  }));

  // Stable ordering: by sidebar_position, then route.
  docs.sort((a, b) => a.order - b.order || a.route.localeCompare(b.route));

  await mkdir(outputDir, { recursive: true });

  // --- llms.txt (summary, llmstxt.org) ---
  const llmsLines = [...HEADER, "## Documentation", ""];

  for (const doc of docs) {
    const url = `${siteUrl}/docs/${doc.route}`;
    const desc = doc.description ? `: ${doc.description}` : "";
    // llmstxt.org: `- [Title](URL): Optional description`
    llmsLines.push(`- [${doc.title}](${url})${desc}`);
  }

  llmsLines.push(
    "",
    "## Links",
    "",
    "- [GitHub](https://github.com/freeoxide/gpui-query): Source code and issues",
    "- [Introduction](https://gpui-query.freeoxide.com/docs/): What is gpui-query and why you need it",
  );

  const llmsTxt = llmsLines.join("\n");
  await writeFile(join(outputDir, "llms.txt"), llmsTxt, "utf-8");
  console.log(`Generated llms.txt (${docs.length} docs, ${llmsTxt.split("\n").length} lines)`);

  // --- llms-full.txt (concatenated content) ---
  const fullLines = [...HEADER];

  for (const doc of docs) {
    fullLines.push(`## ${doc.title}`, "");
    if (doc.description) {
      fullLines.push(`*${doc.description}*`, "");
    }
    fullLines.push(doc.plain, "");
  }

  const llmsFullTxt = fullLines.join("\n");
  await writeFile(join(outputDir, "llms-full.txt"), llmsFullTxt, "utf-8");
  console.log(
    `Generated llms-full.txt (${docs.length} docs, ${llmsFullTxt.split("\n").length} lines)`,
  );
}

main().catch((err: unknown) => {
  console.error("Failed to generate llms.txt:", err);
  process.exit(1);
});
