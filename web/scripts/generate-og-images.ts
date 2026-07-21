#!/usr/bin/env node
/**
 * Generate every OG image for the site (1200x630 PNGs, template in
 * scripts/lib/og.ts, rasterized with sharp):
 *
 *   public/og-image.png        root brand card (homepage, link previews)
 *   public/og/blog.png         /blog index
 *   public/og/changelog.png    /changelog (latest version read from ../CHANGELOG.md)
 *   public/og/faq.png          /faq
 *   public/og/blog/<slug>.png  one per MDX post in src/content/blog
 *   public/og/docs.png        docs OG fallback (Astro-owned)
 *
 * Runs first in `bun run build`, ahead of `astro build`.
 *
 * Usage:
 *   node scripts/generate-og-images.ts                 # everything
 *   node scripts/generate-og-images.ts --only <name>   # one image (page key or post slug)
 *   node scripts/generate-og-images.ts --keep-svg      # also write the .svg for design iteration
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve, join, dirname } from "node:path";
import { Buffer } from "node:buffer";
import process from "node:process";
import sharp from "sharp";
import { composeOgSvg, OG_WIDTH, OG_HEIGHT, type OgContent } from "./lib/og.ts";

interface BlogFrontmatter {
  title: string;
  description?: string;
  date?: string;
  author?: string;
  tags: string[];
}

/* ── CLI args ───────────────────────────────────────────────────────── */
const args = process.argv.slice(2);
const argValue = (flag: string): string | undefined => {
  const i = args.indexOf(flag);
  return i !== -1 ? args[i + 1] : undefined;
};
const onlyName = argValue("--only");
const keepSvg = args.includes("--keep-svg");

const contentDir = resolve("src/content/blog");
const publicDir = resolve("public");
const changelogPath = resolve("..", "CHANGELOG.md");

/* ── Frontmatter parsing (same simple YAML shape as lib/blog.ts) ───── */
function parseFrontmatter(source: string, file: string): BlogFrontmatter {
  const match = source.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) throw new Error(`${file}: no frontmatter block found`);
  const block = match[1];
  const scalar = (key: string): string | undefined =>
    block.match(new RegExp(`^${key}:\\s*["']?(.*?)["']?\\s*$`, "m"))?.[1]?.trim();
  const title = scalar("title");
  if (!title) throw new Error(`${file}: frontmatter is missing a title`);
  return {
    title,
    description: scalar("description"),
    date: scalar("date"),
    author: scalar("author"),
    tags: [...block.matchAll(/^\s+-\s+(.+)$/gm)].map((m) => m[1].trim()),
  };
}

function formatDate(iso: string | undefined): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" });
}

/* Latest release from the crate changelog, e.g. { version: "0.1.4", date: "2026-06-17" }. */
async function latestRelease(): Promise<{ version: string; date: string } | undefined> {
  try {
    const changelog = await readFile(changelogPath, "utf8");
    const m = changelog.match(/^## \[(\d+\.\d+\.\d+)\] - (\d{4}-\d{2}-\d{2})/m);
    return m ? { version: m[1], date: m[2] } : undefined;
  } catch {
    return undefined;
  }
}

/* ── Rasterization ──────────────────────────────────────────────────── */
async function renderPng(content: OgContent, outFile: string): Promise<void> {
  const svg = composeOgSvg(content);
  await mkdir(dirname(outFile), { recursive: true });
  if (keepSvg) await writeFile(outFile.replace(/\.png$/, ".svg"), svg);

  // density supersamples the rasterization (2x) so edges stay crisp when
  // social platforms downsample.
  const png = await sharp(Buffer.from(svg), { density: 144 })
    .resize(OG_WIDTH, OG_HEIGHT, { fit: "cover", position: "center" })
    .png()
    .toBuffer();
  await writeFile(outFile, png);
  console.log(`Generated ${outFile.replace(`${process.cwd()}/`, "")} (${OG_WIDTH}x${OG_HEIGHT})`);
}

/* ── Site pages ─────────────────────────────────────────────────────── */
async function sitePages(): Promise<Array<{ name: string; content: OgContent; outFile: string }>> {
  const release = await latestRelease();
  return [
    {
      name: "root",
      outFile: join(publicDir, "og-image.png"),
      content: {
        title: "gpui-query",
        description: "Zero-boilerplate async state management for GPUI, the Rust framework behind the Zed editor.",
        mono: "caching · retry · cancellation · persistence",
        footerLeft: "github.com/freeoxide/gpui-query",
        footerRight: "#rust #gpui #zed",
      },
    },
    {
      name: "docs",
      outFile: join(publicDir, "og", "docs.png"),
      content: {
        kicker: { text: "gpui-query", suffix: "/ docs" },
        title: "Documentation",
        description: "Guides, API reference, and patterns for async state management in GPUI apps.",
        mono: "getting-started · guides · api · advanced",
        footerLeft: "gpui-query.freeoxide.com/docs",
      },
    },
    {
      name: "blog",
      outFile: join(publicDir, "og", "blog.png"),
      content: {
        kicker: { text: "gpui-query", suffix: "/ blog" },
        title: "Blog",
        description: "Deep dives on async state management for GPUI in Rust: cache policies, cooperative cancellation, and project updates.",
        footerLeft: "gpui-query.freeoxide.com/blog",
        footerRight: "#rust #gpui",
      },
    },
    {
      name: "changelog",
      outFile: join(publicDir, "og", "changelog.png"),
      content: {
        kicker: { text: "gpui-query", suffix: "/ changelog" },
        title: "Changelog",
        description: "Release history for gpui-query: every version, every improvement.",
        footerLeft: release ? `latest v${release.version} · ${formatDate(release.date)}` : undefined,
        footerRight: "#releases",
      },
    },
    {
      name: "faq",
      outFile: join(publicDir, "og", "faq.png"),
      content: {
        kicker: { text: "gpui-query", suffix: "/ faq" },
        title: "Frequently Asked Questions",
        description: "Setup, architecture, cancellation, persistence, and pagination — answered.",
        footerLeft: "gpui-query.freeoxide.com/faq",
      },
    },
  ];
}

/* ── Blog posts ─────────────────────────────────────────────────────── */
async function blogPosts(): Promise<Array<{ name: string; content: OgContent; outFile: string }>> {
  const files = (await readdir(contentDir)).filter((f) => f.endsWith(".mdx"));
  const posts = [];
  for (const file of files) {
    const slug = file.replace(/\.mdx$/, "");
    const fm = parseFrontmatter(await readFile(join(contentDir, file), "utf8"), file);
    posts.push({
      name: slug,
      outFile: join(publicDir, "og", "blog", `${slug}.png`),
      content: {
        kicker: { text: "gpui-query", suffix: "/ blog" },
        title: fm.title,
        description: fm.description,
        footerLeft: [fm.author, formatDate(fm.date)].filter(Boolean).join("  ·  "),
        footerRight: fm.tags.map((t) => `#${t}`).join("  ") || undefined,
      } satisfies OgContent,
    });
  }
  return posts;
}

/* ── Main ───────────────────────────────────────────────────────────── */
async function main(): Promise<void> {
  const all = [...(await sitePages()), ...(await blogPosts())];
  const targets = onlyName ? all.filter((t) => t.name === onlyName) : all;
  if (targets.length === 0) {
    const names = all.map((t) => t.name).join(", ");
    throw new Error(`no image named "${onlyName}" — available: ${names}`);
  }
  for (const target of targets) {
    await renderPng(target.content, target.outFile);
  }
}

main().catch((err: unknown) => {
  console.error("Failed to generate OG images:", err instanceof Error ? err.message : err);
  process.exit(1);
});
