#!/usr/bin/env node

/**
 * Builds the Docusaurus docs site (in docs/) and copies the output to
 * web/public/docs.
 *
 * This runs as a standalone Node script (invoked via `node scripts/build-docs.ts`,
 * type-stripped natively on Node 24+) so that it bypasses bun's automatic
 * `npm run` → `bun run` rewriting, which would otherwise break the Docusaurus
 * build (bun cannot resolve the `docusaurus` binary from docs/node_modules/.bin).
 */

import { execSync } from "node:child_process";
import { existsSync, mkdirSync, cpSync, rmSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL(".", import.meta.url)), "../..");
const DOCS_SITE = resolve(ROOT, "docs");
const OUT = resolve(ROOT, "web/public/docs");

// 1. Install docs deps if node_modules is missing
if (!existsSync(resolve(DOCS_SITE, "node_modules"))) {
  console.log("[build-docs] Installing docs dependencies...");
  execSync("npm install", { cwd: DOCS_SITE, stdio: "inherit" });
}

// 2. Build Docusaurus site
console.log("[build-docs] Building Docusaurus site...");
execSync("npx docusaurus build", { cwd: DOCS_SITE, stdio: "inherit" });

// 3. Copy output to web/public/docs
if (existsSync(OUT)) {
  rmSync(OUT, { recursive: true });
}
mkdirSync(OUT, { recursive: true });

const buildDir = resolve(DOCS_SITE, "build");
cpSync(buildDir, OUT, { recursive: true });

console.log("[build-docs] Docs copied to web/public/docs ✓");
