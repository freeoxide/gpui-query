import type { ComponentType, ReactNode } from "react";
import { CodeBlock } from "#/components/code-block";
import { Callout } from "#/components/callout";

interface MDXComponents {
  pre: ComponentType<{ children: ReactNode }>;
  a: ComponentType<{ href?: string; children: ReactNode }>;
  Callout: typeof Callout;
}

export function getMdxComponents(): MDXComponents {
  return {
    pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
    a: ({ href, children }) => {
      if (href?.startsWith("/")) {
        return (
          <a
            href={href}
            className="font-medium text-primary underline underline-offset-4 hover:text-primary/80"
          >
            {children}
          </a>
        );
      }
      return (
        <a
          href={href}
          target="_blank"
          rel="noopener noreferrer"
          className="font-medium text-primary underline underline-offset-4 hover:text-primary/80"
        >
          {children}
        </a>
      );
    },
    Callout,
  };
}
