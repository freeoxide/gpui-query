#!/usr/bin/env node
/**
 * Generate clean .md alternatives for Docusaurus documentation pages.
 * AI crawlers can fetch these directly for plain markdown content.
 *
 * Source: Docusaurus docs at ../docs/docs (all .md and .mdx files).
 * Previously read src/content/, which no longer exists.
 *
 * Output: {output}/docs/{route}.md
 *
 * Usage: node scripts/generate-md-alt.mjs [--output .output/public]
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, join, relative, resolve, sep } from "node:path";

const outputDir = process.argv.includes("--output")
  ? process.argv[process.argv.indexOf("--output") + 1]
  : ".output/public";

// Scripts run from web/, so the Docusaurus docs live one level up.
const docsRoot = resolve("..", "docs", "docs");

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
  if (!fmMatch) return { frontmatter, body: text };

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
  if (route.endsWith("/index")) route = route.slice(0, -"/index".length);
  return route;
}

/**
 * Strip frontmatter and Docusaurus/MDX-specific syntax down to prose
 * markdown. Admonitions become blockquotes; JSX components and imports
 * are removed.
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
    console.log("No Docusaurus docs found, skipping .md generation");
    return;
  }

  const docsOutDir = join(outputDir, "docs");
  await mkdir(docsOutDir, { recursive: true });

  let generated = 0;
  for (const file of files) {
    const raw = await readFile(file, "utf-8");
    const { frontmatter, body } = parseFrontmatter(raw);
    const relPath = relative(docsRoot, file);
    const route = resolveRoute(relPath, frontmatter);
    const plain = toPlainMarkdown(body);

    // route may contain subdirectory segments (e.g. "guides/caching").
    const outFile = join(docsOutDir, `${route || "index"}.md`);
    await mkdir(dirname(outFile), { recursive: true });
    await writeFile(outFile, `${plain}\n`, "utf-8");
    generated++;
  }

  console.log(`Generated ${generated} .md alternative files in ${docsOutDir}`);
}

main().catch((err) => {
  console.error("Failed to generate .md alternatives:", err);
  process.exit(1);
});
