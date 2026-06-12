import { StackIcon, CpuIcon, DatabaseIcon, ArrowRightIcon } from "@phosphor-icons/react";
import { SectionHeading } from "./decor";

const archLayers = [
  {
    index: "L1",
    icon: StackIcon,
    label: "Hook Layer",
    items: "use_query / use_mutation / use_infinite_query",
    tint: "bg-primary/[0.07]",
  },
  {
    index: "L2",
    icon: CpuIcon,
    label: "Client Layer",
    items: "QueryClient / Registry / GC",
    tint: "bg-primary/[0.04]",
  },
  {
    index: "L3",
    icon: DatabaseIcon,
    label: "Core Layer",
    items: "QueryResource / CachePolicy / QueryKey",
    tint: "bg-primary/[0.02]",
  },
];

function Connector() {
  return (
    <div className="relative mx-auto h-9 w-px bg-border" aria-hidden="true">
      <span className="animate-flow-down absolute -left-[1.5px] h-1 w-1 bg-primary" />
    </div>
  );
}

export function ArchitectureSection() {
  return (
    <section className="py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <SectionHeading
          eyebrow="architecture"
          title="Three layers, one direction"
          description="Each layer has a single responsibility. Data flows down, results flow back up."
        />

        <div className="mx-auto mt-14 max-w-2xl">
          {archLayers.map((layer, i) => {
            const Icon = layer.icon;
            return (
              <div key={layer.label} data-reveal data-reveal-delay={String(i + 1)}>
                {i > 0 && <Connector />}

                <div
                  className={`flex items-center gap-5 border border-border border-l-2 border-l-primary ${layer.tint} px-6 py-5`}
                >
                  <span className="font-mono text-xs text-primary/60">{layer.index}</span>
                  <Icon size={20} weight="duotone" className="shrink-0 text-primary" />
                  <div className="min-w-0">
                    <p className="font-display text-sm font-bold tracking-widest uppercase text-foreground">
                      {layer.label}
                    </p>
                    <p className="mt-0.5 truncate font-mono text-sm text-muted-foreground">
                      {layer.items}
                    </p>
                  </div>
                </div>
              </div>
            );
          })}

          <div
            className="mt-10 flex flex-wrap items-center justify-center gap-2 font-mono text-xs text-muted-foreground"
            data-reveal
            data-reveal-delay="4"
          >
            <span className="text-muted-foreground/60">data flows</span>
            <span className="text-primary">use_query</span>
            <ArrowRightIcon size={13} aria-hidden="true" />
            <span className="text-primary/80">QueryClient</span>
            <ArrowRightIcon size={13} aria-hidden="true" />
            <span className="text-primary/60">QueryResource</span>
          </div>
        </div>
      </div>
    </section>
  );
}
