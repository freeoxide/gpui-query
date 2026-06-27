#!/usr/bin/env node
/**
 * Rasterize web/public/og-image.svg -> og-image.png (1200x630) using sharp.
 *
 * The SVG is authored at 1200x630 (viewBox 0 0 1200 630), so we render it
 * pixel-for-pixel with a density that keeps it crisp on HiDPI. The PNG is
 * referenced by og:image / twitter:image meta tags across the routes.
 *
 * Usage:
 *   node scripts/generate-og-image.mjs                # writes to public/
 *   node scripts/generate-og-image.mjs --output dist  # writes to dist/
 */

import { readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve, dirname } from "node:path";
import sharp from "sharp";

// OG image spec: 1200x630 (matches the SVG's intrinsic viewBox).
const WIDTH = 1200;
const HEIGHT = 630;

const outputArgIndex = process.argv.indexOf("--output");
const outputDir = outputArgIndex !== -1 ? process.argv[outputArgIndex + 1] : "public";

// Source SVG lives alongside the other static assets in web/public.
const svgPath = resolve("public/og-image.svg");
const outPath = resolve(outputDir, "og-image.png");

async function main() {
  const svgBuffer = await readFile(svgPath);

  const pngBuffer = await sharp(svgBuffer, {
    // density scales the rasterizer; 2x the CSS pixel width keeps edges crisp
    // when social platforms downsample the 1200px-wide PNG.
    density: 144,
  })
    .resize(WIDTH, HEIGHT, { fit: "cover", position: "center" })
    .png()
    .toBuffer();

  await mkdir(dirname(outPath), { recursive: true });
  await writeFile(outPath, pngBuffer);
  console.log(`Generated og-image.png (${WIDTH}x${HEIGHT}) -> ${outPath}`);
}

main().catch((err) => {
  console.error("Failed to generate OG image:", err);
  process.exit(1);
});
