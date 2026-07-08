/** Floating switcher for comparing landing page previews. Remove once a winner ships. */
const VERSIONS = [
  { id: "v1", label: "V1 signal path", href: "/v1" },
  { id: "v2", label: "V2 control room", href: "/v2" },
  { id: "v3", label: "V3 blueprint", href: "/v3" },
  { id: "v4", label: "V4 combined", href: "/v4" },
] as const;

export function VersionSwitcher({ current }: { current: "v1" | "v2" | "v3" | "v4" }) {
  return (
    <nav
      aria-label="Landing page previews"
      className="fixed bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center border border-border bg-background/95 font-mono text-[11px] shadow-sm backdrop-blur"
    >
      <span className="border-r border-border px-3 py-2 tracking-[0.2em] uppercase text-muted-foreground">
        Preview
      </span>
      {VERSIONS.map((v) => (
        <a
          key={v.id}
          href={v.href}
          aria-current={v.id === current ? "page" : undefined}
          className={`border-r border-border px-3 py-2 transition-colors ${
            v.id === current
              ? "bg-primary/10 text-primary"
              : "text-muted-foreground hover:text-foreground"
          }`}
        >
          {v.label}
        </a>
      ))}
      <a
        href="/"
        className="px-3 py-2 text-muted-foreground transition-colors hover:text-foreground"
      >
        current
      </a>
    </nav>
  );
}
