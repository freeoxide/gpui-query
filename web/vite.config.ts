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
import { readdirSync } from "node:fs";

/**
 * Dynamic /blog/$slug routes are not statically discoverable by the file-based
 * router, and crawlLinks must stay false (crawling would follow the navbar
 * /docs/ link into the static Docusaurus content, which 404s). So enumerate
 * the blog post paths explicitly from the TSX post filenames, resolved
 * relative to this config file (not process.cwd()) for robustness. The list
 * feeds the top-level `pages` option below.
 */
function blogRoutes(): string[] {
  try {
    const dir = new URL("./src/content/blog", import.meta.url);
    return readdirSync(dir)
      .filter((f) => f.endsWith(".tsx"))
      .map((f) => `/blog/${f.replace(/\.tsx$/, "")}`);
  } catch {
    return [];
  }
}

/**
 * The Cloudflare Vite plugin asserts that no `resolve.external` is set on its
 * Worker environment. Vitest sets `resolve.external` for the node/ssr
 * environment by default, so loading the plugin under `vp test` fails config
 * validation with "avoid setting `resolve.external` in your Cloudflare Worker
 * environments". Skip the plugin entirely when running under Vitest; it stays
 * active for `vp dev` / `vp build`.
 */
const isVitest = !!process.env.VITEST;

const config = defineConfig({
  staged: {
    "*": "vp check --fix",
  },
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
      },
      // Explicitly enumerate dynamic /blog/$slug pages — TanStack Start's
      // `pages` array is a TOP-LEVEL option (sibling of `prerender`), not a
      // key inside it.
      pages: blogRoutes().map((path) => ({ path })),
      sitemap: {
        enabled: true,
        host: "https://gpui-query.hmziq.xyz",
      },
    }),
    viteReact(),
  ],
});

export default config;
