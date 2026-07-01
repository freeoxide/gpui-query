#!/usr/bin/env node
/**
 * Generate llms.txt and llms-full.txt for AI agent optimization.
 * Follows the llmstxt.org specification.
 *
 * Source: Docusaurus docs at ../website/docs (all .md and .mdx files).
 * Previously read src/content/, which no longer exists.
 *
 * Usage: node scripts/generate-llms-txt.mjs [--output .output/public]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { join, relative, resolve, sep } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

// Scripts run from web/, so the Docusaurus docs live one level up.
const docsRoot = resolve("..", "website", "docs");
const siteUrl = "https://gpui-query.freeoxide.com";

/**
 * Recursively collect every .md and .mdx file under `dir`.
 * Returns absolute paths.
 */
async function collectDocs(dir) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return [];
  }

  const files = [];
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectDocs(full)));
    } else if (entry.name.endsWith(".mdx") || entry.name.endsWith(".md")) {
      files.push(full);
    }
  }
  return files;
}

/**
 * Parse a YAML frontmatter block delimited by leading `---` fences.
 * Only handles the flat `key: value` shape used by the docs.
 */
function parseFrontmatter(text) {
  const fmMatch = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  const frontmatter = {};
  if (!fmMatch) return { frontmatter, body: text.replace(/^﻿/, "") };

  for (const line of fmMatch[1].split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const idx = trimmed.indexOf(":");
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim();
    let value = trimmed
      .slice(idx + 1)
      .trim()
      .replace(/^["']|["']$/g, "");
    frontmatter[key] = value;
  }

  const body = text.slice(fmMatch[0].length).replace(/^\r?\n/, "");
  return { frontmatter, body };
}

/**
 * Resolve the Docusaurus route for a doc file.
 * Mirrors Docusaurus' default routing: the route is the file path relative
 * to docs/, without extension, using forward slashes, with index.* -> "" and
 * honoring an explicit `slug:` frontmatter override.
 */
function resolveRoute(relPath, frontmatter) {
  if (frontmatter.slug) {
    // slug: / -> "" (root); otherwise strip leading slash.
    if (frontmatter.slug === "/") return "";
    return frontmatter.slug.replace(/^\/+/, "");
  }

  let route = relPath
    .replace(/\.(mdx?|md)$/, "")
    .split(sep)
    .join("/");
  // index files map to their directory route.
  if (route.endsWith("/index")) route = route.slice(0, -"/index".length);
  // Top-level intro.mdx has no slug override by default; keep as "intro".
  return route;
}

/**
 * Strip frontmatter and Docusaurus/MDX-specific syntax down to prose
 * markdown suitable for llms-full.txt.
 */
function toPlainMarkdown(body) {
  return (
    body
      // Drop ES import / export statements (MDX).
      .replace(/^\s*import\s+.*$/gm, "")
      .replace(/^\s*export\s+.*$/gm, "")
      // Docusaurus admonitions :::type[title] {props} ... :::
      // -> keep inner content as a blockquote.
      .replace(
        /:::[A-Za-z]+(?:\[([^\]]*)\])?(?:\s*\{[^}]*\})?\r?\n([\s\S]*?):::/g,
        (_m, title, inner) => {
          const header = title ? `> **${title.trim()}**\n>\n` : "";
          const quoted = String(inner)
            .replace(/\r?\n$/, "")
            .split(/\r?\n/)
            .map((line) => `> ${line}`)
            .join("\n");
          return `${header}${quoted}`;
        },
      )
      // Any stray admonition fences (opening/closing) without a body.
      .replace(/:::[A-Za-z]+(?:\[([^\]]*)\])?(?:\s*\{[^}]*\})?\s*(\r?\n)?/g, "")
      .replace(/^:::\s*$/gm, "")
      // JSX self-closing and paired tags -> remove (children kept for paired).
      .replace(/<[A-Z][A-Za-z0-9]*[^>]*\/>/g, "")
      .replace(/<\/?[A-Z][A-Za-z0-9]*[^>]*>/g, "")
      // Remove remaining inline JSX expressions like {variable} (best effort).
      .replace(/^\s*\{[^}]*\}\s*$/gm, "")
      // Collapse 3+ blank lines.
      .replace(/\n{3,}/g, "\n\n")
      .trim()
  );
}

async function main() {
  const files = await collectDocs(docsRoot);
  if (files.length === 0) {
    console.log("No Docusaurus docs found, skipping llms.txt generation");
    return;
  }

  const docs = [];
  for (const file of files) {
    const raw = await readFile(file, "utf-8");
    const { frontmatter, body } = parseFrontmatter(raw);
    const relPath = relative(docsRoot, file);
    const route = resolveRoute(relPath, frontmatter);
    docs.push({
      relPath,
      route,
      title: frontmatter.title || route || "Home",
      description: frontmatter.description || "",
      order: Number(frontmatter.sidebar_position ?? 0),
      plain: toPlainMarkdown(body),
    });
  }

  // Stable ordering: by sidebar_position, then route.
  docs.sort((a, b) => a.order - b.order || a.route.localeCompare(b.route));

  await mkdir(outputDir, { recursive: true });

  // --- llms.txt (summary, llmstxt.org) ---
  const llmsLines = [
    "# gpui-query",
    "",
    "> Zero-boilerplate async state management for GPUI. Brings TanStack Query patterns to Rust and the Zed editor's GPUI framework with caching, retry, cooperative cancellation, and persistence.",
    "",
    "## Documentation",
    "",
  ];

  for (const doc of docs) {
    const url = `${siteUrl}/docs/${doc.route}`;
    const title = doc.title;
    const desc = doc.description ? `: ${doc.description}` : "";
    // llmstxt.org: `- [Title](URL): Optional description`
    llmsLines.push(`- [${title}](${url})${desc}`);
  }

  llmsLines.push("");
  llmsLines.push("## Links");
  llmsLines.push("");
  llmsLines.push("- [GitHub](https://github.com/freeoxide/gpui-query): Source code and issues");
  llmsLines.push(
    "- [Introduction](https://gpui-query.freeoxide.com/docs/): What is gpui-query and why you need it",
  );

  const llmsTxt = llmsLines.join("\n");
  await writeFile(join(outputDir, "llms.txt"), llmsTxt, "utf-8");
  console.log(`Generated llms.txt (${docs.length} docs, ${llmsTxt.split("\n").length} lines)`);

  // --- llms-full.txt (concatenated content) ---
  const fullLines = [
    "# gpui-query",
    "",
    "> Zero-boilerplate async state management for GPUI. Brings TanStack Query patterns to Rust and the Zed editor's GPUI framework with caching, retry, cooperative cancellation, and persistence.",
    "",
  ];

  for (const doc of docs) {
    fullLines.push(`## ${doc.title}`);
    fullLines.push("");
    if (doc.description) {
      fullLines.push(`*${doc.description}*`);
      fullLines.push("");
    }
    fullLines.push(doc.plain);
    fullLines.push("");
  }

  const llmsFullTxt = fullLines.join("\n");
  await writeFile(join(outputDir, "llms-full.txt"), llmsFullTxt, "utf-8");
  console.log(
    `Generated llms-full.txt (${docs.length} docs, ${llmsFullTxt.split("\n").length} lines)`,
  );
}

main().catch((err) => {
  console.error("Failed to generate llms.txt:", err);
  process.exit(1);
});
