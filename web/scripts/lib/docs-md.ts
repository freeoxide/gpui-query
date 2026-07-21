/**
 * Shared helpers for scripts that read the Starlight docs
 * (src/content/docs/docs) and turn them into plain markdown:
 * generate-llms-txt.ts and generate-md-alt.ts.
 */

import { readdir, readFile } from "node:fs/promises";
import { join, relative, sep } from "node:path";

export interface DocFrontmatter {
  [key: string]: string;
}

export interface ParsedDoc {
  /** Path relative to the docs root, e.g. "guides/caching.mdx". */
  relPath: string;
  /** Docusaurus route without leading slash, e.g. "guides/caching". "" is the root. */
  route: string;
  frontmatter: DocFrontmatter;
  /** Body stripped down to prose markdown (no MDX/JSX/admonition syntax). */
  plain: string;
}

/** Recursively collect every .md and .mdx file under `dir`, as absolute paths. */
export async function collectDocs(dir: string): Promise<string[]> {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return [];
  }

  const files: string[] = [];
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
export function parseFrontmatter(text: string): { frontmatter: DocFrontmatter; body: string } {
  const fmMatch = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  const frontmatter: DocFrontmatter = {};
  if (!fmMatch) return { frontmatter, body: text.replace(/^﻿/, "") };

  for (const line of fmMatch[1].split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const idx = trimmed.indexOf(":");
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim();
    const value = trimmed
      .slice(idx + 1)
      .trim()
      .replace(/^["']|["']$/g, "");
    frontmatter[key] = value;
  }

  const body = text.slice(fmMatch[0].length).replace(/^\r?\n/, "");
  return { frontmatter, body };
}

/**
 * Resolve the docs route for a doc file (relative to the Starlight content dir
 * src/content/docs/docs). Mirrors Starlight/Astro routing: the route is the
 * file path without extension, using forward slashes, with index.* -> "" (the
 * /docs/ root) and nested index.* -> their directory route.
 */
export function resolveRoute(relPath: string, frontmatter: DocFrontmatter): string {
  if (frontmatter.slug) {
    // slug: / -> "" (root); otherwise strip leading slash.
    if (frontmatter.slug === "/") return "";
    return frontmatter.slug.replace(/^\/+/, "");
  }

  let route = relPath
    .replace(/\.(mdx?|md)$/, "")
    .split(sep)
    .join("/");
  // index files map to their directory route (bare index -> root "").
  if (route === "index") return "";
  if (route.endsWith("/index")) route = route.slice(0, -"/index".length);
  return route;
}

/**
 * Strip frontmatter and Docusaurus/MDX-specific syntax down to prose
 * markdown. Admonitions become blockquotes; JSX components and imports
 * are removed.
 */
export function toPlainMarkdown(body: string): string {
  return (
    body
      // Drop ES import / export statements (MDX).
      .replace(/^\s*import\s+.*$/gm, "")
      .replace(/^\s*export\s+.*$/gm, "")
      // Docusaurus admonitions :::type[title] {props} ... :::
      // -> keep inner content as a blockquote.
      .replace(
        /:::[A-Za-z]+(?:\[([^\]]*)\])?(?:\s*\{[^}]*\})?\r?\n([\s\S]*?):::/g,
        (_m, title: string | undefined, inner: string) => {
          const header = title ? `> **${title.trim()}**\n>\n` : "";
          const quoted = inner
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

/** Read and parse every doc under `docsRoot`. */
export async function loadDocs(docsRoot: string): Promise<ParsedDoc[]> {
  const files = await collectDocs(docsRoot);
  const docs: ParsedDoc[] = [];
  for (const file of files) {
    const raw = await readFile(file, "utf-8");
    const { frontmatter, body } = parseFrontmatter(raw);
    const relPath = relative(docsRoot, file);
    docs.push({
      relPath,
      route: resolveRoute(relPath, frontmatter),
      frontmatter,
      plain: toPlainMarkdown(body),
    });
  }
  return docs;
}
