import { defineConfig } from "vite-plus";
import { devtools } from "@tanstack/devtools-vite";

import { tanstackStart } from "@tanstack/react-start/plugin/vite";

import viteReact from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { cloudflare } from "@cloudflare/vite-plugin";
import mdx from "@mdx-js/rollup";
import rehypePrettyCode from "rehype-pretty-code";
import remarkGfm from "remark-gfm";
import remarkFrontmatter from "remark-frontmatter";
import remarkMdxFrontmatter from "remark-mdx-frontmatter";
import { readdirSync, readFileSync } from "node:fs";
import { LANDING_PREVIEWS_ENABLED } from "./src/lib/flags";

/**
 * Dynamic /blog/$slug routes are not statically discoverable by the file-based
 * router, and crawlLinks must stay false (crawling would follow the navbar
 * /docs/ link into the static Docusaurus content, which 404s). So enumerate
 * the blog post paths explicitly from the MDX post filenames, resolved
 * relative to this config file (not process.cwd()) for robustness. The list
 * feeds the top-level `pages` option below. Each post's frontmatter date
 * becomes its sitemap lastmod, so entries don't all claim the build date.
 */
function blogPages(): Array<{ path: string; sitemap?: { lastmod: string } }> {
  try {
    const dir = new URL("./src/content/blog", import.meta.url);
    return readdirSync(dir)
      .filter((f) => f.endsWith(".mdx"))
      .map((f) => {
        const path = `/blog/${f.replace(/\.mdx$/, "")}`;
        const source = readFileSync(new URL(`./src/content/blog/${f}`, import.meta.url), "utf8");
        const date = source.match(/^date:\s*["']?(\d{4}-\d{2}-\d{2})/m)?.[1];
        return date ? { path, sitemap: { lastmod: date } } : { path };
      });
  } catch {
    return [];
  }
}

/**
 * The Cloudflare Vite plugin asserts that no `resolve.external` is set on its
 * Worker environment. Vitest sets `resolve.external` for the node/ssr
 * environment by default, so loading the plugin under `vp test` fails config
 * validation. Skip the plugin entirely when running under Vitest; it stays
 * active for `vp dev` / `vp build`.
 */
const isVitest = !!process.env.VITEST;

const config = defineConfig({
  lint: { options: { typeAware: true, typeCheck: true } },
  resolve: { tsconfigPaths: true },
  plugins: [
    devtools(),
    ...(isVitest ? [] : [cloudflare({ viteEnvironment: { name: "ssr" }, assetsOnly: true })]),
    tailwindcss(),
    mdx({
      remarkPlugins: [remarkGfm, remarkFrontmatter, remarkMdxFrontmatter],
      rehypePlugins: [[rehypePrettyCode, { theme: "github-dark-default" }]],
    }),
    tanstackStart({
      prerender: {
        enabled: true,
        crawlLinks: false,
        autoStaticPathsDiscovery: true,
        // Emit flat files (faq.html) instead of directory indexes
        // (faq/index.html). With directory indexes, Cloudflare serves pages
        // at the trailing-slash URL and 308s the non-slash URL — but every
        // canonical tag and sitemap entry uses the non-slash form, so Google
        // saw all canonical URLs as redirects and kept pages out of the
        // index ("crawled, currently not indexed").
        autoSubfolderIndex: false,
        // While the landing previews are disabled their routes throw
        // notFound(), so skip them at prerender time too. Also drop the
        // auto-discovered "/blog/" index page: it would emit blog/index.html
        // (a directory index served at the trailing-slash URL), while the
        // canonical/sitemap scheme is non-slash. The explicit "/blog" entry
        // in `pages` below replaces it and emits a flat blog.html.
        filter: (page) =>
          (LANDING_PREVIEWS_ENABLED || !/^\/v[1-4]$/.test(page.path)) && page.path !== "/blog/",
      },
      // Explicitly enumerate dynamic /blog/$slug pages — TanStack Start's
      // `pages` array is a TOP-LEVEL option (sibling of `prerender`), not a
      // key inside it.
      pages: [
        { path: "/blog" },
        // The auto-discovered "/blog/" (trailing-slash) variant is dropped
        // from prerendering by the filter above, but sitemap generation still
        // picks it up from route discovery — exclude it there too so only the
        // non-slash /blog is advertised.
        { path: "/blog/", sitemap: { exclude: true } },
        ...blogPages(),
        // The /v1–/v4 landing previews are noindex; keep them out of the
        // sitemap so it never advertises pages that refuse indexing.
        ...["/v1", "/v2", "/v3", "/v4"].map((path) => ({
          path,
          sitemap: { exclude: true },
        })),
      ],
      sitemap: {
        enabled: true,
        host: "https://gpui-query.freeoxide.com",
      },
    }),
    viteReact(),
  ],
});

export default config;
