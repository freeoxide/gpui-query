import { Link, useRouter } from "@tanstack/react-router";
import { cn } from "#/lib/utils";

interface NavItem {
  slug: string;
  title: string;
  category?: string;
}

interface DocsSidebarProps {
  items: NavItem[];
}

export function DocsSidebar({ items }: DocsSidebarProps) {
  const router = useRouter();
  const currentPath = router.state.location.pathname;

  // Group items by category
  const grouped = items.reduce<Record<string, NavItem[]>>((acc, item) => {
    const category = item.category ?? "Documentation";
    if (!acc[category]) acc[category] = [];
    acc[category].push(item);
    return acc;
  }, {});

  return (
    <aside className="hidden w-64 shrink-0 md:block">
      <nav
        className="sticky top-16 max-h-[calc(100vh-4rem)] overflow-y-auto py-6 pr-4"
        aria-label="Documentation sidebar"
      >
        {Object.entries(grouped).map(([category, categoryItems]) => (
          <div key={category} className="mb-4">
            <h3 className="mb-2 px-2 text-sm font-semibold text-muted-foreground">{category}</h3>
            <ul className="space-y-1">
              {categoryItems.map((item) => {
                const isActive = currentPath === `/docs/${item.slug}`;
                return (
                  <li key={item.slug}>
                    <Link
                      to="/docs/$slug"
                      params={{ slug: item.slug }}
                      className={cn(
                        "block rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-accent hover:text-accent-foreground",
                        isActive && "bg-accent text-accent-foreground font-medium",
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
    </aside>
  );
}
