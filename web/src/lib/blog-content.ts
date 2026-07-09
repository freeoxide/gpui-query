import { type ComponentType } from "react";

/**
 * Compiled MDX post bodies. Import this ONLY from code-split route
 * components — anything reachable from a route loader/head lives in the
 * entry chunk, and a static path to these modules would ship every post
 * body to every page (metadata belongs in `blog.ts`). The glob stays
 * `eager` for the same prerender reason documented there.
 */
const modules = import.meta.glob<{ default: ComponentType }>("../content/blog/*.mdx", {
  eager: true,
});

/** The rendered MDX component for a post, by slug. */
export function getBlogContent(slug: string): ComponentType | undefined {
  return modules[`../content/blog/${slug}.mdx`]?.default;
}
