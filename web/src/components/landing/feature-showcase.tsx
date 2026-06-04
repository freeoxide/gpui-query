import {
  LightningIcon,
  ArrowsLeftRightIcon,
  InfinityIcon,
  DatabaseIcon,
  XCircleIcon,
  HardDriveIcon,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";

interface Feature {
  icon: ReactNode;
  title: string;
  description: string;
  span?: string;
  extra?: ReactNode;
}

const snippetCode = `let query = use_query(cx, "users", || async {
  fetch_users().await
});`;

const features: Feature[] = [
  {
    icon: <LightningIcon size={20} weight="duotone" />,
    title: "Declarative Queries",
    description:
      "Write queries declaratively. gpui-query handles fetching, caching, and state updates automatically.",
    span: "sm:col-span-2",
    extra: (
      <div className="mt-4 bg-muted/50 px-4 py-3 font-mono text-xs leading-relaxed text-muted-foreground">
        <pre className="whitespace-pre">{snippetCode}</pre>
      </div>
    ),
  },
  {
    icon: <ArrowsLeftRightIcon size={20} weight="duotone" />,
    title: "Mutations",
    description:
      "First-class mutation support with success/error callbacks and optimistic updates.",
  },
  {
    icon: <InfinityIcon size={20} weight="duotone" />,
    title: "Infinite Queries",
    description:
      "Paginate effortlessly with built-in infinite query support and bidirectional fetching.",
  },
  {
    icon: <XCircleIcon size={20} weight="duotone" />,
    title: "Cancellation",
    description:
      "Signal-checked retries and cooperative cancellation via Arc<AtomicBool> for clean async lifecycle management.",
  },
  {
    icon: <HardDriveIcon size={20} weight="duotone" />,
    title: "Persistence",
    description:
      "Serialize and restore query state with custom persistence backends.",
  },
  {
    icon: <DatabaseIcon size={20} weight="duotone" />,
    title: "Smart Caching",
    description:
      "TTL, Stale-While-Revalidate, LatestWins, IgnoreWhileLoading — cache policies for every use case.",
    span: "sm:col-span-2",
    extra: (
      <div className="mt-4 flex flex-wrap gap-2">
        {["TTL", "SWR", "LatestWins", "IgnoreWhileLoading"].map((policy) => (
          <span
            key={policy}
            className="border border-primary/20 bg-primary/5 px-2.5 py-1 text-xs font-medium tracking-wide text-primary"
          >
            {policy}
          </span>
        ))}
      </div>
    ),
  },
];

export function FeatureShowcase() {
  return (
    <section className="border-t border-border bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        {/* Left-aligned header */}
        <div data-reveal>
          <p className="text-xs font-medium tracking-widest uppercase text-primary">
            Capabilities
          </p>
          <h2 className="mt-2 text-3xl font-bold tracking-tight sm:text-4xl">
            Built for how you actually write GPUI code
          </h2>
        </div>

        {/* Asymmetric bento grid */}
        <div className="mt-14 grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {features.map((feature, i) => (
            <div
              key={feature.title}
              data-reveal
              data-reveal-delay={String(i + 1)}
              className={`group relative bg-card px-6 py-8 ring-1 ring-foreground/8 transition-colors duration-300 hover:ring-primary/25 ${feature.span ?? ""}`}
            >
              {/* Subtle top edge glow */}
              <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/20 to-transparent" />

              <div className="flex h-8 w-8 items-center justify-center text-primary">
                {feature.icon}
              </div>
              <h3 className="mt-4 text-sm font-semibold tracking-wide uppercase">
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
    </section>
  );
}
