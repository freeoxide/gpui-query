"use client";

import { useEffect, useRef } from "react";
import { Link, useRouter } from "@tanstack/react-router";
import { cn } from "#/lib/utils";

interface NavItem {
  slug: string;
  title: string;
  category?: string;
}

interface DocsSidebarProps {
  items: NavItem[];
  mobileOpen?: boolean;
  onMobileClose?: () => void;
}

export function DocsSidebar({ items, mobileOpen, onMobileClose }: DocsSidebarProps) {
  const router = useRouter();
  const currentPath = router.state.location.pathname;
  const activeRef = useRef<HTMLAnchorElement>(null);

  // Group items by category
  const grouped = items.reduce<Record<string, NavItem[]>>((acc, item) => {
    const category = item.category ?? "Documentation";
    if (!acc[category]) acc[category] = [];
    acc[category].push(item);
    return acc;
  }, {});

  // Scroll active link into view on mount and when path changes
  useEffect(() => {
    if (activeRef.current) {
      activeRef.current.scrollIntoView({
        block: "nearest",
        behavior: "smooth",
      });
    }
  }, [currentPath]);

  // Lock body scroll when mobile sidebar is open
  useEffect(() => {
    if (mobileOpen) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [mobileOpen]);

  // Close mobile sidebar on route change
  useEffect(() => {
    onMobileClose?.();
  }, [currentPath, onMobileClose]);

  const sidebarNav = (
    <nav
      className={cn(
        "max-h-[calc(100vh-3.5rem)] overflow-y-auto py-6 pr-4",
        mobileOpen !== undefined && "px-4 pr-6 pt-2",
      )}
      aria-label="Documentation sidebar"
    >
      {Object.entries(grouped).map(([category, categoryItems]) => (
        <div key={category} className="mb-5">
          <h3 className="mb-2 px-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {category}
          </h3>
          <ul className="space-y-0.5">
            {categoryItems.map((item) => {
              const isActive = currentPath === `/docs/${item.slug}`;
              return (
                <li key={item.slug}>
                  <Link
                    ref={isActive ? activeRef : undefined}
                    to="/docs/$slug"
                    params={{ slug: item.slug }}
                    className={cn(
                      "block border-l-2 py-1.5 text-sm",
                      isActive
                        ? "border-primary bg-primary/5 pl-3 font-medium text-primary"
                        : "border-transparent pl-3 text-muted-foreground transition-colors duration-150 hover:bg-accent/50 hover:text-accent-foreground",
                    )}
                  >
                    {item.title}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );

  return (
    <>
      {/* Desktop sidebar */}
      <aside className="hidden w-64 shrink-0 md:block">
        <div className="sticky top-16">{sidebarNav}</div>
      </aside>

      {/* Mobile overlay */}
      {mobileOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-40 bg-black/50 md:hidden"
            onClick={onMobileClose}
            aria-hidden="true"
          />
          {/* Sidebar sheet */}
          <aside className="fixed inset-y-0 left-0 z-50 w-72 bg-background shadow-xl md:hidden">
            <div className="flex h-14 items-center border-b px-4">
              <span className="text-sm font-semibold">Docs</span>
              <button
                onClick={onMobileClose}
                className="ml-auto rounded-md p-1.5 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                aria-label="Close sidebar"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M18 6 6 18" />
                  <path d="m6 6 12 12" />
                </svg>
              </button>
            </div>
            <div className="overflow-y-auto">{sidebarNav}</div>
          </aside>
        </>
      )}
    </>
  );
}
