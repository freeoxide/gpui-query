#!/usr/bin/env node
/**
 * Generate per-post OG images (1200x630 PNG) for every MDX file in
 * src/content/blog. Each image is composed as an SVG from the post's
 * frontmatter (title, description, date, author, tags) using the shared
 * design tokens (near-black background, lime primary, square corners, the
 * left-accent-bar header motif from the blog pages), then rasterized with
 * sharp — same pipeline as scripts/generate-og-image.ts.
 *
 * Output: public/og/blog/<slug>.png, referenced by og:image / twitter:image
 * and the BlogPosting JSON-LD in src/routes/blog/$slug.tsx. Runs as part of
 * `bun run build` (before `vp build`, so the PNGs are copied into dist as
 * static assets); rerun manually after editing a post's frontmatter.
 *
 * Usage:
 *   node scripts/generate-blog-og-images.ts                 # all posts -> public/
 *   node scripts/generate-blog-og-images.ts --only <slug>   # one post
 *   node scripts/generate-blog-og-images.ts --output dist/client
 *   node scripts/generate-blog-og-images.ts --keep-svg      # also write the .svg for design iteration
 */

import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve, join } from "node:path";
import { Buffer } from "node:buffer";
import process from "node:process";
import sharp from "sharp";

const WIDTH = 1200;
const HEIGHT = 630;

/* ── Shared design tokens (see shared/tokens.css, dark mode) ───────── */
const BG = "#0a0a0a"; // --color-background: oklch(0.145 0 0)
const FG = "#fafafa"; // --color-foreground: oklch(0.985 0 0)
const MUTED = "#a1a1a1"; // --color-muted-foreground: oklch(0.708 0 0)
const PRIMARY = "#84cc16"; // --color-primary: oklch(0.768 0.233 130.85)
const GRID = "rgba(255,255,255,0.05)"; // --color-border: oklch(1 0 0 / 10%), halved for subtlety

// librsvg resolves locally installed fonts; Inter/JetBrains Mono are webfonts
// on the site, so fall back through common system equivalents.
const SANS = `'Inter', 'Helvetica Neue', 'Arial', sans-serif`;
const MONO = `'JetBrains Mono', 'Menlo', 'Consolas', monospace`;

interface BlogFrontmatter {
  title: string;
  description?: string;
  date?: string;
  author?: string;
  tags: string[];
}

interface TitleLayout {
  size: number;
  lines: string[];
}

/* ── CLI args ───────────────────────────────────────────────────────── */
const args = process.argv.slice(2);
const argValue = (flag: string): string | undefined => {
  const i = args.indexOf(flag);
  return i !== -1 ? args[i + 1] : undefined;
};
const outputDir = argValue("--output") ?? "public";
const onlySlug = argValue("--only");
const keepSvg = args.includes("--keep-svg");

const contentDir = resolve("src/content/blog");
const outDir = resolve(outputDir, "og/blog");

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

const escapeXml = (s: string): string => s.replace(/[<>&"']/g, (c) => `&#${c.charCodeAt(0)};`);

/* ── Text measurement + wrapping ────────────────────────────────────── */
/*
 * SVG <text> doesn't wrap, so lines are broken here. Widths are estimated
 * from per-character factors (em units) — coarse, but the layout leaves
 * enough slack that a ±5% miss doesn't clip.
 */
function charFactor(ch: string): number {
  if ("iljt!.,':;|·".includes(ch)) return 0.3;
  if (ch === " ") return 0.28;
  if ("frI-()[]{}".includes(ch)) return 0.4;
  if ("mwMW@".includes(ch)) return 0.94;
  if (ch >= "A" && ch <= "Z") return 0.72;
  return 0.56;
}

function estimateWidth(text: string, fontSize: number, bold = false): number {
  let em = 0;
  for (const ch of text) em += charFactor(ch);
  return em * fontSize * (bold ? 1.06 : 1.0);
}

function wrapText(
  text: string,
  fontSize: number,
  maxWidth: number,
  maxLines: number,
  bold = false,
): { lines: string[]; truncated: boolean } {
  const words = text.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    if (estimateWidth(candidate, fontSize, bold) <= maxWidth || !current) {
      current = candidate;
    } else {
      lines.push(current);
      current = word;
    }
  }
  if (current) lines.push(current);
  if (lines.length > maxLines) {
    const kept = lines.slice(0, maxLines);
    kept[maxLines - 1] = `${kept[maxLines - 1].replace(/[.,;:]?$/, "")}…`;
    return { lines: kept, truncated: true };
  }
  return { lines, truncated: false };
}

/* Pick the largest title size whose wrap fits the line budget. */
function layoutTitle(title: string, maxWidth: number): TitleLayout {
  for (const { size, maxLines } of [
    { size: 66, maxLines: 2 },
    { size: 56, maxLines: 2 },
    { size: 48, maxLines: 3 },
  ]) {
    const wrapped = wrapText(title, size, maxWidth, maxLines, true);
    if (!wrapped.truncated) return { size, lines: wrapped.lines };
  }
  // Very long titles: 3 lines at 48px, ellipsized.
  return { size: 48, lines: wrapText(title, 48, maxWidth, 3, true).lines };
}

/* ── SVG composition ────────────────────────────────────────────────── */
function composeSvg(fm: BlogFrontmatter): string {
  const MARGIN = 80;
  const BAR_W = 8; // left accent bar, mirrors border-l-4 on the blog header
  const TEXT_X = MARGIN + BAR_W + 28;
  const MAX_TEXT_W = WIDTH - TEXT_X - MARGIN;

  const title = layoutTitle(fm.title, MAX_TEXT_W);
  const titleLineHeight = Math.round(title.size * 1.16);

  const parts: string[] = [];

  // Background + subtle vertical grid (the site's line-based theme).
  parts.push(`<rect width="${WIDTH}" height="${HEIGHT}" fill="${BG}"/>`);
  for (let x = 150; x < WIDTH; x += 150) {
    parts.push(
      `<line x1="${x}" y1="0" x2="${x}" y2="${HEIGHT}" stroke="${GRID}" stroke-width="1"/>`,
    );
  }
  parts.push(
    `<line x1="0" y1="${HEIGHT / 2}" x2="${WIDTH}" y2="${HEIGHT / 2}" stroke="${GRID}" stroke-width="1"/>`,
  );

  // Kicker: project + section, in mono like the site's code accents.
  parts.push(
    `<text x="${MARGIN}" y="128" font-family="${MONO}" font-size="26" fill="${PRIMARY}">gpui-query <tspan fill="${MUTED}">/ blog</tspan></text>`,
  );

  // Magnifier logo mark (navbar logo: circle + handle), top right.
  parts.push(
    `<g stroke="${PRIMARY}" stroke-width="4" fill="none" stroke-linecap="round">` +
      `<circle cx="${WIDTH - MARGIN - 34}" cy="112" r="24"/>` +
      `<line x1="${WIDTH - MARGIN - 16}" y1="130" x2="${WIDTH - MARGIN}" y2="146"/>` +
      `</g>`,
  );

  // Title block with the left accent bar.
  const titleTop = 208;
  const firstBaseline = titleTop + Math.round(title.size * 0.78);
  title.lines.forEach((line, i) => {
    parts.push(
      `<text x="${TEXT_X}" y="${firstBaseline + i * titleLineHeight}" font-family="${SANS}" font-size="${title.size}" font-weight="700" fill="${FG}">${escapeXml(line)}</text>`,
    );
  });
  const titleBottom =
    firstBaseline + (title.lines.length - 1) * titleLineHeight + Math.round(title.size * 0.24);
  parts.push(
    `<rect x="${MARGIN}" y="${titleTop - 4}" width="${BAR_W}" height="${titleBottom - titleTop + 8}" fill="${PRIMARY}"/>`,
  );

  // Description, muted, up to two lines.
  if (fm.description) {
    const desc = wrapText(fm.description, 28, MAX_TEXT_W, 2);
    const descBaseline = titleBottom + 56;
    desc.lines.forEach((line, i) => {
      parts.push(
        `<text x="${TEXT_X}" y="${descBaseline + i * 42}" font-family="${SANS}" font-size="28" fill="${MUTED}">${escapeXml(line)}</text>`,
      );
    });
  }

  // Footer: author · date on the left, tags on the right.
  const footerY = HEIGHT - MARGIN + 10;
  const byline = [fm.author, formatDate(fm.date)].filter(Boolean).join("  ·  ");
  if (byline) {
    parts.push(
      `<text x="${MARGIN}" y="${footerY}" font-family="${SANS}" font-size="26" font-weight="600" fill="${FG}">${escapeXml(byline)}</text>`,
    );
  }
  if (fm.tags.length > 0) {
    const tagText = fm.tags.map((t) => `#${t}`).join("  ");
    parts.push(
      `<text x="${WIDTH - MARGIN}" y="${footerY}" text-anchor="end" font-family="${MONO}" font-size="24" fill="${PRIMARY}" opacity="0.85">${escapeXml(tagText)}</text>`,
    );
  }

  // Bottom accent bar in the primary lime.
  parts.push(`<rect x="0" y="${HEIGHT - 10}" width="${WIDTH}" height="10" fill="${PRIMARY}"/>`);

  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}">` +
    parts.join("\n") +
    `</svg>`
  );
}

/* ── Main ───────────────────────────────────────────────────────────── */
async function main(): Promise<void> {
  const files = (await readdir(contentDir)).filter((f) => f.endsWith(".mdx"));
  const targets = onlySlug ? files.filter((f) => f === `${onlySlug}.mdx`) : files;
  if (targets.length === 0) {
    throw new Error(
      onlySlug ? `no post named ${onlySlug}.mdx in src/content/blog` : "no MDX posts found",
    );
  }

  await mkdir(outDir, { recursive: true });

  for (const file of targets) {
    const slug = file.replace(/\.mdx$/, "");
    const source = await readFile(join(contentDir, file), "utf8");
    const fm = parseFrontmatter(source, file);
    const svg = composeSvg(fm);

    if (keepSvg) await writeFile(join(outDir, `${slug}.svg`), svg);

    // density supersamples the rasterization (2x, like generate-og-image.ts)
    // so edges stay crisp when social platforms downsample.
    const png = await sharp(Buffer.from(svg), { density: 144 })
      .resize(WIDTH, HEIGHT, { fit: "cover", position: "center" })
      .png()
      .toBuffer();
    await writeFile(join(outDir, `${slug}.png`), png);
    console.log(`Generated og/blog/${slug}.png (${WIDTH}x${HEIGHT})`);
  }
}

main().catch((err: unknown) => {
  console.error("Failed to generate blog OG images:", err instanceof Error ? err.message : err);
  process.exit(1);
});
