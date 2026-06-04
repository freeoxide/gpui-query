declare module "*.mdx" {
  import type { ComponentType } from "react";

  const Content: ComponentType;
  export default Content;

  export const frontmatter: {
    title: string;
    description: string;
    category?: string;
    tags?: string[];
    date?: string;
    author?: string;
    order?: number;
  };
}
