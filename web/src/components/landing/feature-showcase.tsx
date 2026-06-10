import {
  LightningIcon,
  ArrowsLeftRightIcon,
  InfinityIcon,
  DatabaseIcon,
  XCircleIcon,
  HardDriveIcon,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { CornerMarks, SectionHeading } from "./decor";

interface Feature {
  icon: ReactNode;
  title: string;
  description: string;
  extra?: ReactNode;
}

const features: Feature[] = [
  {
    icon: <LightningIcon size={22} weight="duotone" />,
    title: "Declarative Queries",
    description:
      "One hook. gpui-query handles fetching, caching, and state updates so your render code stays a pure function of data.",
  },
  {
    icon: <DatabaseIcon size={22} weight="duotone" />,
    title: "Smart Caching",
    description: "Cache policies for every use case — pick one per query and move on.",
    extra: (
      <div className="mt-4 flex flex-wrap gap-2">
        {["TTL", "SWR", "LatestWins", "IgnoreWhileLoading"].map((policy) => (
          <span
            key={policy}
            className="border border-primary/20 bg-primary/5 px-2.5 py-1 font-mono text-[11px] tracking-wide text-primary"
          >
            {policy}
          </span>
        ))}
      </div>
    ),
  },
  {
    icon: <ArrowsLeftRightIcon size={22} weight="duotone" />,
    title: "Mutations",
    description:
      "First-class mutation support with success and error callbacks, plus optimistic updates that roll back on failure.",
  },
  {
    icon: <InfinityIcon size={22} weight="duotone" />,
    title: "Infinite Queries",
    description:
      "Paginate effortlessly with built-in infinite query support and bidirectional fetching.",
  },
  {
    icon: <XCircleIcon size={22} weight="duotone" />,
    title: "Cancellation",
    description:
      "Signal-checked retries and cooperative cancellation for a clean async lifecycle — no orphaned tasks.",
  },
  {
    icon: <HardDriveIcon size={22} weight="duotone" />,
    title: "Persistence",
    description:
      "Serialize and restore query state across launches with custom persistence backends.",
  },
];

export function FeatureShowcase() {
  return (
    <section className="border-b border-border py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <SectionHeading
          eyebrow="capabilities"
          title="Everything the fetch deserves"
          description="The full TanStack Query feature set, rebuilt around GPUI's entity and async model."
        />

        {/* Seamless 1px grid */}
        <div className="relative mt-14" data-reveal data-reveal-delay="1">
          <CornerMarks />
          <div className="grid grid-cols-1 gap-px border border-border bg-border sm:grid-cols-2 lg:grid-cols-3">
            {features.map((feature, i) => (
              <div
                key={feature.title}
                className="group relative bg-background p-7 transition-colors duration-300 hover:bg-card"
              >
                <span className="absolute top-6 right-6 font-mono text-xs text-muted-foreground/50 transition-colors duration-300 group-hover:text-primary">
                  {String(i + 1).padStart(2, "0")}
                </span>
                <div className="text-primary">{feature.icon}</div>
                <h3 className="font-display mt-5 text-base font-semibold tracking-wide">
                  {feature.title}
                </h3>
                <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                  {feature.description}
                </p>
                {feature.extra}
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
