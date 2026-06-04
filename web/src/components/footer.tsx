import { Link } from "@tanstack/react-router";
import { Github } from "lucide-react";

export function Footer() {
  return (
    <footer className="border-t bg-background">
      <div className="mx-auto flex max-w-7xl flex-col items-center gap-4 px-4 py-8 sm:flex-row sm:justify-between sm:px-6 lg:px-8">
        <div className="flex flex-col items-center gap-2 sm:items-start">
          <Link to="/" className="text-sm font-semibold">
            gpui-query
          </Link>
          <p className="text-xs text-muted-foreground">&copy; 2026 hmziq. All rights reserved.</p>
        </div>
        <nav
          className="flex items-center gap-4 text-sm text-muted-foreground"
          aria-label="Footer navigation"
        >
          <Link to="/docs" className="hover:text-foreground transition-colors">
            Docs
          </Link>
          <Link to="/blog" className="hover:text-foreground transition-colors">
            Blog
          </Link>
          <Link to="/faq" className="hover:text-foreground transition-colors">
            FAQ
          </Link>
          <a
            href="https://github.com/hmziqrs/gpui-query"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-foreground transition-colors"
            aria-label="GitHub repository"
          >
            <Github className="h-4 w-4" />
          </a>
        </nav>
      </div>
    </footer>
  );
}
