import { Link } from "@tanstack/react-router";
import { Menu, Github, Search } from "lucide-react";
import { Button } from "#/components/ui/button";
import { ThemeToggle } from "#/components/theme-toggle";
import { MobileNav } from "#/components/mobile-nav";
import { SearchDialog } from "#/components/search-dialog";
import { useState, useEffect } from "react";

const navLinks = [
  { href: "/docs", label: "Docs" },
  { href: "/blog", label: "Blog" },
  { href: "/faq", label: "FAQ" },
];

export function Navbar() {
  const [mobileOpen, setMobileOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  // Scroll shadow effect
  useEffect(() => {
    function handleScroll() {
      setScrolled(window.scrollY > 10);
    }
    window.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll();
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  // Cmd+K / Ctrl+K shortcut
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setSearchOpen((prev) => !prev);
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <>
      <header
        className={`sticky top-0 z-40 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 transition-shadow duration-300${
          scrolled ? " shadow-sm" : ""
        }`}
      >
        <div className="mx-auto flex h-14 max-w-7xl items-center px-4 sm:px-6 lg:px-8">
          <Link to="/" className="mr-6 flex items-center space-x-2">
            <svg viewBox="0 0 32 32" className="h-7 w-7 text-primary">
              <circle cx="16" cy="16" r="14" fill="none" stroke="currentColor" strokeWidth="2" />
              <path d="M22 22l4 4" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
              <path
                d="M16 10v12M10 16h12"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                opacity="0.5"
              />
            </svg>
            <span className="text-xl font-bold">gpui-query</span>
          </Link>

          <nav className="hidden flex-1 items-center gap-6 md:flex" aria-label="Main navigation">
            {navLinks.map((link) => (
              <Link
                key={link.href}
                to={link.href}
                className="text-sm font-medium text-foreground/70 transition-colors hover:text-foreground relative after:absolute after:bottom-[-2px] after:left-0 after:h-0.5 after:w-0 after:bg-primary after:transition-all after:duration-200 hover:after:w-full"
                activeOptions={{ exact: false }}
                activeProps={{ className: "text-primary" }}
              >
                {link.label}
              </Link>
            ))}
          </nav>

          <div className="flex flex-1 items-center justify-end gap-2 md:flex">
            <Button
              variant="outline"
              size="sm"
              className="hidden h-8 gap-2 text-muted-foreground md:flex"
              onClick={() => setSearchOpen(true)}
            >
              <Search className="h-3.5 w-3.5" />
              <span className="text-xs">Search</span>
              <kbd className="pointer-events-none ml-1 rounded border bg-muted px-1 py-0.5 text-[10px]">
                ⌘K
              </kbd>
            </Button>
            <ThemeToggle />
            <Button variant="ghost" size="icon" asChild className="hidden md:inline-flex">
              <a
                href="https://github.com/hmziqrs/gpui-query"
                target="_blank"
                rel="noopener noreferrer"
                aria-label="GitHub repository"
              >
                <Github className="h-5 w-5" />
              </a>
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="md:hidden"
              onClick={() => setMobileOpen(true)}
              aria-label="Open navigation menu"
            >
              <Menu className="h-5 w-5" />
            </Button>
          </div>
        </div>

        <MobileNav open={mobileOpen} onClose={() => setMobileOpen(false)} />
      </header>

      <SearchDialog open={searchOpen} onClose={() => setSearchOpen(false)} />
    </>
  );
}
