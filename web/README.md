# gpui-query website

Marketing site, blog, and Starlight documentation for
[gpui-query](https://github.com/freeoxide/gpui-query), built with
[Astro](https://astro.build) + [Starlight](https://starlight.astro.build).
Static output deployed to Cloudflare Pages.

## Stack

- **Astro 7** (static output) — marketing/blog/legal pages in `src/pages/**`,
  Starlight docs in `src/content/docs/docs/**` served at `/docs/**`.
- **Starlight** — documentation.
- **React islands** — interactive pieces (navbar, theme toggle, search, landing)
  hydrated via `client:*` directives.
- **Tailwind CSS v4** + shared design tokens in `../shared/tokens.css`.
- **Pagefind** — combined site search (post-build index of marketing + docs).

## Commands

```bash
bun install          # install deps
bun run dev          # astro dev server on :3000
bun run build        # og images → astro build → pagefind → llms.txt/.md alts
bun run preview      # preview the built site
bun run deploy       # build + deploy dist/client to Cloudflare Pages
```

## Structure

- `src/pages/**` — Astro routes (`index`, `blog`, `blog/[slug]`, `faq`,
  `changelog`, `privacy`, `terms`, `404`).
- `src/content/docs/docs/**` — Starlight documentation (served at `/docs/**`).
- `src/content/blog/**` — blog posts (MDX).
- `src/layouts/BaseLayout.astro` — app shell (metadata, theme, navbar/footer,
  analytics).
- `src/styles/starlight.css` — Starlight theme mapped onto shared tokens.
- `src/components/**` — React islands + UI components.
- `scripts/**` — build-time generators (OG images, llms.txt, .md alternatives).

## URLs

Canonical URLs have no trailing slash (e.g. `/blog`, `/docs/api/queries`).
Astro's `build.format: "file"` emits flat `*.html` so Cloudflare Pages
(`html_handling: "auto-trailing-slash"`) serves them directly without redirects.
