/**
 * Shared OG-image SVG template for generate-og-images.ts.
 *
 * Composes a 1200x630 social card from the shared design tokens
 * (see shared/tokens.css, dark mode): near-black background with a subtle
 * line grid, lime primary accents, square corners, and the left-accent-bar
 * header motif used on the blog/FAQ pages. Rasterized with sharp by the
 * caller.
 */

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const OG_WIDTH = 1200;
export const OG_HEIGHT = 630;

/* ── Shared design tokens (see shared/tokens.css, dark mode) ───────── */
const BG = "#0a0a0a"; // --color-background: oklch(0.145 0 0)
const FG = "#fafafa"; // --color-foreground: oklch(0.985 0 0)
const MUTED = "#a1a1a1"; // --color-muted-foreground: oklch(0.708 0 0)
const PRIMARY = "#9AE600"; // brand lime — shared/tokens.css --color-primary (light)
const GRID = "rgba(255,255,255,0.05)"; // --color-border: oklch(1 0 0 / 10%), halved for subtlety

// librsvg resolves locally installed fonts; Inter/JetBrains Mono are webfonts
// on the site, so fall back through common system equivalents.
const SANS = `'Inter', 'Helvetica Neue', 'Arial', sans-serif`;
const MONO = `'JetBrains Mono', 'Menlo', 'Consolas', monospace`;

// The real brand mark, inlined into OG cards. Read from the canonical logo SVG
// (web/public/logo.svg) so social cards stay in sync with the navbar/footer
// mark — no hand-redrawn copy to drift. Strips the outer <svg> wrapper; the
// inner paths are in 0..1024 viewBox coords (scaled at the embed site).
const LOGO_MARK = (() => {
  const svg = readFileSync(
    resolve(dirname(fileURLToPath(import.meta.url)), "../../public/logo.svg"),
    "utf8",
  );
  return svg.replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>[\s\S]*$/, "");
})();

export interface OgContent {
  /** Top-left label, e.g. { text: "gpui-query", suffix: "/ blog" }. */
  kicker?: { text: string; suffix?: string };
  title: string;
  description?: string;
  /** Mono lime line under the description, e.g. "caching · retry · cancellation". */
  mono?: string;
  /** Bottom-left, sans semibold white, e.g. "author · date". */
  footerLeft?: string;
  /** Bottom-right, mono lime, e.g. "#rust #gpui". */
  footerRight?: string;
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
function layoutTitle(title: string, maxWidth: number): { size: number; lines: string[] } {
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
export function composeOgSvg(content: OgContent): string {
  const MARGIN = 80;
  const BAR_W = 8; // left accent bar, mirrors border-l-4 on the blog header
  const TEXT_X = MARGIN + BAR_W + 28;
  const MAX_TEXT_W = OG_WIDTH - TEXT_X - MARGIN;

  const title = layoutTitle(content.title, MAX_TEXT_W);
  const titleLineHeight = Math.round(title.size * 1.16);

  const parts: string[] = [];

  // Background + subtle vertical grid (the site's line-based theme).
  parts.push(`<rect width="${OG_WIDTH}" height="${OG_HEIGHT}" fill="${BG}"/>`);
  for (let x = 150; x < OG_WIDTH; x += 150) {
    parts.push(
      `<line x1="${x}" y1="0" x2="${x}" y2="${OG_HEIGHT}" stroke="${GRID}" stroke-width="1"/>`,
    );
  }
  parts.push(
    `<line x1="0" y1="${OG_HEIGHT / 2}" x2="${OG_WIDTH}" y2="${OG_HEIGHT / 2}" stroke="${GRID}" stroke-width="1"/>`,
  );

  // Kicker: project + section, in mono like the site's code accents.
  if (content.kicker) {
    const suffix = content.kicker.suffix
      ? ` <tspan fill="${MUTED}">${escapeXml(content.kicker.suffix)}</tspan>`
      : "";
    parts.push(
      `<text x="${MARGIN}" y="128" font-family="${MONO}" font-size="26" fill="${PRIMARY}">${escapeXml(content.kicker.text)}${suffix}</text>`,
    );
  }

  // Brand logo mark (same glyph as navbar/footer), top right. The mark fills a
  // 1024-box; scale it into a 72px box flush with the top-right margin.
  const MARK_PX = 72;
  parts.push(
    `<g transform="translate(${OG_WIDTH - MARGIN - MARK_PX} ${112 - MARK_PX / 2}) scale(${MARK_PX / 1024})">${LOGO_MARK}</g>`,
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

  // Description, muted, up to two lines; optional mono accent line below.
  let blockBottom = titleBottom;
  if (content.description) {
    const desc = wrapText(content.description, 28, MAX_TEXT_W, 2);
    const descBaseline = titleBottom + 56;
    desc.lines.forEach((line, i) => {
      parts.push(
        `<text x="${TEXT_X}" y="${descBaseline + i * 42}" font-family="${SANS}" font-size="28" fill="${MUTED}">${escapeXml(line)}</text>`,
      );
    });
    blockBottom = descBaseline + (desc.lines.length - 1) * 42;
  }
  if (content.mono) {
    parts.push(
      `<text x="${TEXT_X}" y="${blockBottom + 54}" font-family="${MONO}" font-size="24" fill="${PRIMARY}" opacity="0.9">${escapeXml(content.mono)}</text>`,
    );
  }

  // Footer: sans byline on the left, mono tags on the right.
  const footerY = OG_HEIGHT - MARGIN + 10;
  if (content.footerLeft) {
    parts.push(
      `<text x="${MARGIN}" y="${footerY}" font-family="${SANS}" font-size="26" font-weight="600" fill="${FG}">${escapeXml(content.footerLeft)}</text>`,
    );
  }
  if (content.footerRight) {
    parts.push(
      `<text x="${OG_WIDTH - MARGIN}" y="${footerY}" text-anchor="end" font-family="${MONO}" font-size="24" fill="${PRIMARY}" opacity="0.85">${escapeXml(content.footerRight)}</text>`,
    );
  }

  // Bottom accent bar in the primary lime.
  parts.push(`<rect x="0" y="${OG_HEIGHT - 10}" width="${OG_WIDTH}" height="10" fill="${PRIMARY}"/>`);

  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${OG_WIDTH}" height="${OG_HEIGHT}" viewBox="0 0 ${OG_WIDTH} ${OG_HEIGHT}">` +
    parts.join("\n") +
    `</svg>`
  );
}
