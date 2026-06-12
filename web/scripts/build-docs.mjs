#!/usr/bin/env node

/**
 * Builds the Docusaurus docs website and copies the output to web/public/docs.
 *
 * This runs as a standalone Node script (invoked via `node scripts/build-docs.mjs`)
 * so that it bypasses bun's automatic `npm run` → `bun run` rewriting, which
 * would otherwise break the Docusaurus build (bun cannot resolve the `docusaurus`
 * binary from website/node_modules/.bin).
 */

import { execSync } from "node:child_process";
import { existsSync, mkdirSync, cpSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const ROOT = resolve(import.meta.dirname, "../..");
const WEBSITE = resolve(ROOT, "website");
const OUT = resolve(ROOT, "web/public/docs");

// 1. Install website deps if node_modules is missing
if (!existsSync(resolve(WEBSITE, "node_modules"))) {
  console.log("[build-docs] Installing website dependencies...");
  execSync("npm install", { cwd: WEBSITE, stdio: "inherit" });
}

// 2. Build Docusaurus site
console.log("[build-docs] Building Docusaurus site...");
execSync("npx docusaurus build", { cwd: WEBSITE, stdio: "inherit" });

// 3. Copy output to web/public/docs
if (existsSync(OUT)) {
  rmSync(OUT, { recursive: true });
}
mkdirSync(OUT, { recursive: true });

const buildDir = resolve(WEBSITE, "build");
cpSync(buildDir, OUT, { recursive: true });

console.log("[build-docs] Docs copied to web/public/docs ✓");
