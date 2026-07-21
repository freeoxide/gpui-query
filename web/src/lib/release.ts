/**
 * Build-time source of truth for the crate version and changelog.
 *
 * The version lives in `crates/gpui-query/Cargo.toml`; the changelog lives in
 * the repo-root `CHANGELOG.md`. This module reads both at build time so the
 * website never hardcodes a duplicate that can drift.
 *
 * Paths resolve from `process.cwd()`, which is `web/` for both consumers —
 * the `astro build` (SSG) and the bare-`node` prebuild scripts (the OG script
 * already relies on this, e.g. `resolve("src/content/blog")`). We deliberately
 * do NOT use `import.meta.url`: Astro bundles this module into a prerender
 * chunk under `dist/`, where `import.meta.url` points at the built chunk, not
 * the source file.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(process.cwd(), "..");

export type Category = "Added" | "Changed" | "Fixed" | "Removed";

export interface ChangelogItem {
  category: Category;
  text: string;
}

export interface ChangelogEntry {
  /** Version without a leading "v", e.g. "0.2.0". */
  version: string;
  /** ISO date, e.g. "2026-07-21". */
  date: string;
  /** One-line summary from the blockquote under the version header, if any. */
  description?: string;
  items: ChangelogItem[];
}

/**
 * Read the main crate's version from its Cargo manifest. This is the canonical
 * version number; the release workflow keeps it in sync with `CHANGELOG.md`.
 */
export function crateVersion(): string {
  const toml = readFileSync(
    resolve(REPO_ROOT, "crates/gpui-query/Cargo.toml"),
    "utf8",
  );
  const m = toml.match(/^version = "([^"]+)"/m);
  return m ? m[1] : "0.0.0";
}

const VERSION_HEADER = /^## \[(\d+\.\d+\.\d+)\] - (\d{4}-\d{2}-\d{2})/;
const CATEGORY = /^### (Added|Changed|Fixed|Removed)\b/;
const BLOCKQUOTE = /^>\s?(.*)$/;
const ITEM = /^[-*]\s+(.+)$/;

/**
 * Parse the repo-root `CHANGELOG.md` (Keep a Changelog format) into entries.
 *
 * Each `## [x.y.z] - YYYY-MM-DD` header starts an entry. An optional blockquote
 * line (`> summary`) before the first `### ` becomes the `description`.
 * `### Added|Changed|Fixed|Removed` sections collect their `- ` bullets as
 * items. `####` sub-headings, blank lines, and the trailing link-reference
 * footer are ignored. Versions without a date (e.g. an `## [Unreleased]`)
 * are skipped.
 */
export function parseChangelog(): ChangelogEntry[] {
  const md = readFileSync(resolve(REPO_ROOT, "CHANGELOG.md"), "utf8");
  const lines = md.split(/\r?\n/);
  const entries: ChangelogEntry[] = [];
  let cur: ChangelogEntry | null = null;
  let category: Category | null = null;
  let descriptionDone = false;

  for (const line of lines) {
    const head = line.match(VERSION_HEADER);
    if (head) {
      if (cur) entries.push(cur);
      cur = { version: head[1], date: head[2], items: [] };
      category = null;
      descriptionDone = false;
      continue;
    }
    if (!cur) continue; // preamble / link footer before the first version
    if (/^## /.test(line)) {
      // A non-dated `##` header (e.g. Unreleased): close the current entry.
      entries.push(cur);
      cur = null;
      continue;
    }
    const cat = line.match(CATEGORY);
    if (cat) {
      category = cat[1] as Category;
      descriptionDone = true;
      continue;
    }
    if (!descriptionDone) {
      const bq = line.match(BLOCKQUOTE);
      if (bq && bq[1].trim()) {
        cur.description = bq[1].trim();
        descriptionDone = true;
        continue;
      }
    }
    const item = line.match(ITEM);
    if (item && category) {
      cur.items.push({ category, text: item[1].trim() });
    }
  }
  if (cur) entries.push(cur);
  return entries;
}

/**
 * The latest release for OG/SEO use: the version from Cargo.toml paired with
 * the date of the top changelog entry. Returns `undefined` if either source
 * can't be read.
 */
export function latestRelease(): { version: string; date: string } | undefined {
  try {
    const entries = parseChangelog();
    if (entries.length === 0) return undefined;
    return { version: crateVersion(), date: entries[0].date };
  } catch {
    return undefined;
  }
}
