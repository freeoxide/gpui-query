# gpui-query brand assets

Single source of truth for the project mark and its favicon/icon derivatives.

## The mark
`combined.svg` is the master (black on transparent): a 6-lobe atom/flower with a
central hexagonal **gpui** gear. It is the design source — do **not** hand-edit
the derivatives; regenerate them.

## Canonical logo
`logo.svg` / `logo.png` — the mark in the brand lime (`#9AE600`) on a
**transparent** background. No tile, no light/dark variants — one accent logo.

Used everywhere:
- `web/public/logo.svg` + `web/public/logo.png` — in-page mark (navbar, footer)
  and the README (`web/public/logo.png`)
- `web/public/favicon.svg` — vector favicon (same mark)
- rasterized → `favicon.ico`, `apple-touch-icon.png`, `icon-192/512.png` (manifest)
- inlined into `web/scripts/lib/og.ts` for OG images (read at build time)
- `web/src/assets/logo.svg` — Starlight docs sidebar logo

## Brand color (from `shared/tokens.css`)
| token | oklch | hex | use |
|---|---|---|---|
| `--color-primary` (light) | `oklch(0.841 0.238 128.85)` | `#9AE600` | mark fill |
| `--color-primary` (dark) | `oklch(0.768 0.233 130.85)` | `#7CCF00` | (alt shade) |

## Regenerate
Requires `rsvg-convert` (`brew install librsvg`):

```sh
node brand/generate.mjs
```

This rebuilds `logo.svg` + `exports/*` **and** re-deploys the web-facing files
into `web/public/` (and `web/src/assets/logo.svg`). Re-run after editing
`combined.svg`.

## exports/
- `logo.png` — 512 px transparent PNG (the "simple logo", README asset)
- `favicon.ico` — 16/32/48 px transparent (PNG-in-ICO)
- `apple-touch-icon.png` — 180 px (iOS home screen)
- `icon-192.png`, `icon-512.png` — PWA manifest (`purpose: any`)
- `favicon-{16,32,48,180,192,512,1024}.png`, `icon-1024.png` — size archive
