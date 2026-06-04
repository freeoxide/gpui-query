import { Link } from "@tanstack/react-router";
import { Github } from "lucide-react";

const docLinks = [
  { label: "Getting Started", slug: "getting-started" },
  { label: "Core Concepts", slug: "core-concepts" },
  { label: "API", slug: "api-reference" },
] as const;

export function Footer() {
  return (
    <footer className="border-t border-teal-900/20 bg-[#0a0f14]">
      <div className="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
        {/* Logo + tagline */}
        <div className="mb-10 flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-teal-600 text-white">
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
          <span className="text-lg font-bold tracking-tight text-white">gpui-query</span>
        </div>

        {/* 3-column grid */}
        <div className="grid grid-cols-1 gap-8 sm:grid-cols-3">
          {/* Documentation */}
          <div>
            <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-teal-400">
              Documentation
            </h3>
            <ul className="space-y-2.5">
              {docLinks.map(({ label, slug }) => (
                <li key={slug}>
                  <Link
                    to="/docs/$slug"
                    params={{ slug }}
                    className="text-sm text-slate-400 transition-all duration-200 hover:text-teal-400 hover:translate-x-0.5"
                  >
                    {label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          {/* Community */}
          <div>
            <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-teal-400">
              Community
            </h3>
            <ul className="space-y-2.5">
              <li>
                <a
                  href="https://github.com/hmziqrs/gpui-query"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-2 text-sm text-slate-400 transition-all duration-200 hover:text-teal-400 hover:translate-x-0.5"
                >
                  <Github className="h-4 w-4" />
                  GitHub
                </a>
              </li>
              <li>
                <Link
                  to="/blog"
                  search={{ tag: undefined }}
                  className="inline-block text-sm text-slate-400 transition-all duration-200 hover:text-teal-400 hover:translate-x-0.5"
                >
                  Blog
                </Link>
              </li>
            </ul>
          </div>

          {/* Legal */}
          <div>
            <h3 className="mb-4 text-xs font-semibold uppercase tracking-wider text-teal-400">
              Legal
            </h3>
            <ul className="space-y-2.5">
              <li>
                <span className="text-sm text-slate-400">MIT License</span>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="mt-10 border-t border-teal-900/20 pt-6 text-center">
          <p className="text-xs text-slate-500">
            &copy; 2024&ndash;2026 hmziq. Built with{" "}
            <a
              href="https://tanstack.com/start"
              target="_blank"
              rel="noopener noreferrer"
              className="text-teal-400 transition-colors duration-200 hover:text-teal-300"
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
