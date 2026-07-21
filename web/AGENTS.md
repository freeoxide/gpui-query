# Website (Astro + Starlight)

This is the gpui-query website: a static Astro + Starlight site deployed to
Cloudflare Pages. The migration from TanStack Start + Docusaurus is complete —
do **not** reintroduce Vite+, TanStack Start, or Docusaurus.

## Commands (run from `web/`)

- `bun install` — install dependencies.
- `bun run dev` — Astro dev server on http://localhost:3000.
- `bun run build` — full production build: OG images → `astro build` → Pagefind
  index → `llms.txt` / `.md` alternatives. Output is `dist/client/`.
- `bun run preview` — serve the built site.
- `astro check` — type-check `.astro` / `.tsx` sources.

## Layout

- `src/pages/**` — Astro routes (marketing, blog, legal).
- `src/content/docs/docs/**` — Starlight docs served at `/docs/**`.
- `src/content/blog/**` — blog posts (MDX).
- `src/layouts/BaseLayout.astro` — app shell; `src/styles/starlight.css` — docs
  theme layered on shared tokens.
- Interactive UI stays as React islands (`client:load` / `client:idle`).
- Package manager is **bun**.

## Conventions

- Canonical URLs are no-trailing-slash; `build.format: "file"` keeps output flat.
- Shared design tokens live in `../shared/tokens.css` — consume them, don't
  recolor.
- Docs internal links must sit under `/docs/**` (root-absolute `/api/...` links
  404).
- Search is one combined Pagefind index (`data-pagefind-body` on the app
  `<main>` and on Starlight content).
