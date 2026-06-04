import { Link } from "@tanstack/react-router";
import { Github } from "lucide-react";

const docLinks = [
  { label: "Getting Started", href: "/docs/getting-started/installation" },
  { label: "Core Concepts", href: "/docs/" },
  { label: "API", href: "/docs/api/queries" },
] as const;

export function Footer() {
  return (
    <footer className="border-t bg-card">
      <div className="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
        {/* Logo + tagline */}
        <div className="mb-10 flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded bg-primary text-primary-foreground">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2.5}
              strokeLinecap="round"
              strokeLinejoin="round"
              className="h-4 w-4"
            >
              <path d="M4 6h16M4 12h10M4 18h14" />
            </svg>
          </div>
          <span className="text-lg font-bold tracking-tight text-foreground">gpui-query</span>
        </div>

        {/* 3-column grid */}
        <div className="grid grid-cols-1 gap-8 sm:grid-cols-3">
          {/* Documentation */}
          <div>
            <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-primary">
              Documentation
            </h3>
            <ul className="space-y-2.5">
              {docLinks.map(({ label, href }) => (
                <li key={href}>
                  <a
                    href={href}
                    className="text-sm text-muted-foreground transition-colors duration-200 hover:text-primary"
                  >
                    {label}
                  </a>
                </li>
              ))}
            </ul>
          </div>

          {/* Community */}
          <div>
            <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-primary">
              Community
            </h3>
            <ul className="space-y-2.5">
              <li>
                <a
                  href="https://github.com/hmziqrs/gpui-query"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors duration-200 hover:text-primary"
                >
                  <Github className="h-4 w-4" />
                  GitHub
                </a>
              </li>
              <li>
                <Link
                  to="/about"
                  className="inline-block text-sm text-muted-foreground transition-colors duration-200 hover:text-primary"
                >
                  About
                </Link>
              </li>
            </ul>
          </div>

          {/* Legal */}
          <div>
            <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-primary">
              Legal
            </h3>
            <ul className="space-y-2.5">
              <li>
                <span className="text-sm text-muted-foreground">MIT License</span>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="mt-10 border-t pt-6 text-center">
          <p className="text-xs text-muted-foreground">
            &copy; 2024&ndash;2026 hmziq. Built with{" "}
            <a
              href="https://tanstack.com/start"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary transition-colors duration-200 hover:text-primary/80"
            >
              TanStack Start
            </a>
            .
          </p>
        </div>
      </div>
    </footer>
  );
}
