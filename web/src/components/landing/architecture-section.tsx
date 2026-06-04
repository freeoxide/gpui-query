import {
  StackIcon,
  CpuIcon,
  DatabaseIcon,
  ArrowRightIcon,
} from "@phosphor-icons/react";

const archLayers = [
  {
    label: "Hook Layer",
    items: "use_query / use_mutation / use_infinite_query",
    color: "primary",
  },
  {
    label: "Client Layer",
    items: "QueryClient / Registry / GC",
    color: "emerald",
  },
  {
    label: "Core Layer",
    items: "QueryResource / CachePolicy / QueryKey",
    color: "teal",
  },
] as const;

const colorMap: Record<string, { border: string; bg: string; text: string }> =
  {
    primary: {
      border: "border-primary/40",
      bg: "bg-primary/5",
      text: "text-primary",
    },
    emerald: {
      border: "border-emerald-500/40",
      bg: "bg-emerald-500/5",
      text: "text-emerald-600 dark:text-emerald-400",
    },
    teal: {
      border: "border-teal-500/40",
      bg: "bg-teal-500/5",
      text: "text-teal-600 dark:text-teal-400",
    },
  };

const layerIcons = [StackIcon, CpuIcon, DatabaseIcon];

export function ArchitectureSection() {
  return (
    <section className="border-t border-border py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div data-reveal>
          <p className="text-xs font-medium tracking-widest uppercase text-primary">
            Architecture
          </p>
          <h2 className="mt-2 text-3xl font-bold tracking-tight sm:text-4xl">
            Three-layer design
          </h2>
          <p className="mt-4 max-w-2xl text-muted-foreground">
            Designed in three clean layers, each with a clear responsibility.
          </p>
        </div>

        <div className="mx-auto mt-14 max-w-2xl">
          {archLayers.map((layer, i) => {
            const Icon = layerIcons[i];
            const colors = colorMap[layer.color];
            return (
              <div key={layer.label} data-reveal data-reveal-delay={String(i + 1)}>
                {/* Connecting line */}
                {i > 0 && (
                  <div className="mx-auto h-8 w-px bg-border" />
                )}

                {/* Layer bar */}
                <div
                  className={`flex items-center gap-4 border-l-4 ${colors.border} ${colors.bg} px-6 py-5 transition-colors duration-200 hover:bg-opacity-100`}
                >
                  <Icon size={20} weight="duotone" className={`shrink-0 ${colors.text}`} />
                  <div className="min-w-0">
                    <p className="text-xs font-bold uppercase tracking-widest text-foreground/80">
                      {layer.label}
                    </p>
                    <p className={`mt-0.5 truncate font-mono text-sm ${colors.text}`}>
                      {layer.items}
                    </p>
                  </div>
                </div>
              </div>
            );
          })}

          {/* Data flow summary */}
          <div className="mt-10 flex items-center justify-center gap-2 text-sm text-muted-foreground">
            <span className="font-medium">Data flows</span>
            <span className="text-primary">use_query</span>
            <ArrowRightIcon size={14} className="text-muted-foreground" />
            <span className="text-emerald-600 dark:text-emerald-400">
              QueryClient
            </span>
            <ArrowRightIcon size={14} className="text-muted-foreground" />
            <span className="text-teal-600 dark:text-teal-400">
              QueryResource
            </span>
          </div>
        </div>
      </div>
    </section>
  );
}
