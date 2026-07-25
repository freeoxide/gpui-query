// brand/generate.mjs — regenerate the gpui-query brand mark + favicon set.
//
// Source of truth: brand/combined.svg (black mark). This script:
//   1. builds brand/logo.svg      — lime mark (#9AE600), TRANSPARENT background
//   2. rasterizes it → PNGs (16/32/48/180/192/512/1024) via rsvg-convert (transparent)
//   3. packs favicon.ico (16/32/48, PNG-in-ICO, transparent)
//   4. emits brand/exports/*  AND  deploys the web-facing files into web/public/
//      (+ web/src/assets/logo.svg for the Starlight sidebar)
//
// Run:  `node brand/generate.mjs`  (or `bun brand/generate.mjs`)
// Needs: `rsvg-convert` (`brew install librsvg`).
//
// Brand color from shared/tokens.css: --color-primary (light)
//   oklch(0.841 0.238 128.85) → #9AE600

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync, existsSync, copyFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
const SRC = join(HERE, "combined.svg");
const EXPORTS = join(HERE, "exports");
const WEB_PUBLIC = join(ROOT, "web", "public");
const WEB_ASSETS = join(ROOT, "web", "src", "assets");
mkdirSync(EXPORTS, { recursive: true });

const LIME = "#9AE600";

if (!existsSync(SRC)) {
  console.error(`✗ source not found: ${SRC}`);
  process.exit(1);
}
const combined = readFileSync(SRC, "utf8");

// Recolor every black fill → lime (handles #000000 and shorthand #000).
// No background rect — the logo is a transparent lime mark.
const logoSvg = combined
  .replace(/#000000/gi, LIME)
  .replace(/#000(?![0-9a-fA-F])/gi, LIME);

writeFileSync(join(HERE, "logo.svg"), logoSvg);
console.log(`✓ brand/logo.svg  (transparent lime mark)`);

// Rasterize (transparent — no -b background flag).
const sizes = [16, 32, 48, 180, 192, 512, 1024];
const png = (px) => join(EXPORTS, `favicon-${px}.png`);
for (const px of sizes) {
  execFileSync("rsvg-convert", ["-w", String(px), "-h", String(px), join(HERE, "logo.svg"), "-o", png(px)], {
    stdio: "inherit",
  });
}
console.log(`✓ rasterized ${sizes.join(", ")}px (transparent)`);

// Canonical PNG logo (512) — the "simple logo in png format", also for the README.
copyFileSync(png(512), join(EXPORTS, "logo.png"));

// favicon.ico — 16/32/48 transparent PNGs packed as PNG-in-ICO.
const icoSizes = [16, 32, 48];
const ico = packIco(icoSizes.map((px) => ({ px, data: readFileSync(png(px)) })));
writeFileSync(join(EXPORTS, "favicon.ico"), ico);
console.log(`✓ exports/{favicon.ico, logo.png}`);

// Named copies consumers expect.
copyFileSync(png(180), join(EXPORTS, "apple-touch-icon.png"));
copyFileSync(png(192), join(EXPORTS, "icon-192.png"));
copyFileSync(png(512), join(EXPORTS, "icon-512.png"));
copyFileSync(png(1024), join(EXPORTS, "icon-1024.png"));

// ── Deploy into web/ ────────────────────────────────────────────────
// logo.svg lives in HERE; raster derivatives live in EXPORTS.
const deploy = [
  [join(HERE, "logo.svg"), "logo.svg"],
  [join(EXPORTS, "logo.png"), "logo.png"],
  [join(HERE, "logo.svg"), "favicon.svg"], // vector favicon = same transparent mark
  [join(EXPORTS, "favicon.ico"), "favicon.ico"],
  [join(EXPORTS, "apple-touch-icon.png"), "apple-touch-icon.png"],
  [join(EXPORTS, "icon-192.png"), "icon-192.png"],
  [join(EXPORTS, "icon-512.png"), "icon-512.png"],
];
for (const [from, to] of deploy) {
  copyFileSync(from, join(WEB_PUBLIC, to));
}
copyFileSync(join(HERE, "logo.svg"), join(WEB_ASSETS, "logo.svg")); // Starlight sidebar
console.log(`✓ deployed logo + favicon set to web/public/ and web/src/assets/`);

console.log("\nDone. README can reference the logo at: web/public/logo.png");

// Minimal PNG-in-ICO encoder. Layout: ICONDIR(6) + N×ICONDIRENTRY(16) + PNG blobs.
function packIco(imgs) {
  const count = imgs.length;
  const header = 6;
  let offset = header + 16 * count;
  const entries = [];
  for (const { px, data } of imgs) {
    const w = px >= 256 ? 0 : px; // 0 == 256
    const entry = Buffer.alloc(16);
    entry.writeUInt8(w, 0);
    entry.writeUInt8(w, 1);
    entry.writeUInt8(0, 2); // color count
    entry.writeUInt8(0, 3); // reserved
    entry.writeUInt16LE(1, 4); // planes
    entry.writeUInt16LE(32, 6); // bpp
    entry.writeUInt32LE(data.length, 8); // size
    entry.writeUInt32LE(offset, 12); // offset
    entries.push(entry);
    offset += data.length;
  }
  const dir = Buffer.alloc(header);
  dir.writeUInt16LE(0, 0); // reserved
  dir.writeUInt16LE(1, 2); // type = icon
  dir.writeUInt16LE(count, 4);
  return Buffer.concat([dir, ...entries, ...imgs.map((i) => i.data)]);
}
