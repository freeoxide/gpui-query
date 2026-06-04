import { createFileRoute, Link } from "@tanstack/react-router";
import {
  Zap,
  ArrowRightLeft,
  Infinity as InfinityIcon,
  Database,
  XCircle,
  HardDrive,
  ArrowRight,
  Github,
  Check,
  X,
  Layers,
  Cpu,
  Code2,
  Terminal,
} from "lucide-react";
import { Button } from "#/components/ui/button";
import { Card, CardHeader, CardContent } from "#/components/ui/card";
import { CodeBlock } from "#/components/code-block";
import { softwareSourceCode } from "#/lib/seo";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "gpui-query — Async State Management for GPUI" },
      {
        name: "description",
        content:
          "Zero-boilerplate async state management for GPUI. Caching, retry, cooperative cancellation, and persistence for the Zed editor's framework.",
      },
      { property: "og:title", content: "gpui-query — Async State Management for GPUI" },
      {
        property: "og:description",
        content:
          "Zero-boilerplate async state management for GPUI. Caching, retry, cooperative cancellation, and persistence.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://gpui-query.hmziq.xyz" },
      { property: "og:image", content: "https://gpui-query.hmziq.xyz/og-image.svg" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "gpui-query — Async State Management for GPUI" },
      { name: "twitter:description", content: "Zero-boilerplate async state management for GPUI." },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz" }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify(
          softwareSourceCode({
            name: "gpui-query",
            description: "Zero-boilerplate async state management for GPUI",
            programmingLanguage: "Rust",
            codeRepository: "https://github.com/hmziqrs/gpui-query",
            url: "https://gpui-query.hmziq.xyz",
            license: "MIT",
          }),
        ),
      },
    ],
  }),
  component: Home,
});

function Home() {
  return (
    <div className="flex flex-col">
      <HeroSection />
      <FeatureGrid />
      <QuickStartSection />
      <ComparisonTable />
      <ArchitectureSection />
      <CtaFooterSection />
    </div>
  );
}

/* ─── Hero ─────────────────────────────────────────────────────── */

function HeroSection() {
  return (
    <section className="relative overflow-hidden">
      {/* Animated gradient background */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10"
        style={{
          backgroundSize: "200% 200%",
          animation: "gradient-shift 8s ease infinite",
        }}
      >
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-emerald-500/5" />
      </div>

      {/* Dot grid overlay */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10 opacity-[0.35]"
        style={{
          backgroundImage: "radial-gradient(circle, var(--color-primary) 0.5px, transparent 0.5px)",
          backgroundSize: "24px 24px",
        }}
      />

      {/* Top radial glow */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute left-1/2 top-0 -z-10 h-[500px] w-[900px] -translate-x-1/2 -translate-y-1/3 rounded-full opacity-40 blur-3xl"
        style={{
          background:
            "radial-gradient(ellipse at center, var(--color-primary) 0%, transparent 70%)",
        }}
      />

      <div className="mx-auto max-w-7xl px-4 pb-28 pt-24 sm:px-6 sm:pb-36 sm:pt-32 lg:px-8">
        <div className="mx-auto max-w-3xl text-center">
          {/* Badge */}
          <div className="mb-8 inline-flex items-center gap-2 rounded-full border border-primary/20 bg-primary/5 px-4 py-1.5 text-sm font-medium text-primary">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-primary opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-primary" />
            </span>
            Open-source async state for Rust GPUI
          </div>

          <h1 className="text-4xl font-black tracking-tight sm:text-5xl lg:text-6xl">
            <span
              className="bg-gradient-to-r from-primary via-primary/80 to-emerald-400 bg-clip-text text-transparent"
              style={{
                backgroundSize: "200% auto",
                animation: "gradient-shift 6s ease infinite",
              }}
            >
              Zero-boilerplate
            </span>
            <br />
            <span className="text-foreground">async state for GPUI</span>
          </h1>

          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground sm:text-xl">
            Fetch, cache, and synchronize async data with a single hook. No manual lifecycle
            management. Built for the Zed editor framework.
          </p>

          <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Button
              size="lg"
              asChild
              className="px-6 text-base font-semibold shadow-lg shadow-primary/20"
            >
              <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
                Get Started
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>

            <Button variant="outline" size="lg" asChild className="px-6 text-base font-semibold">
              <a
                href="https://github.com/hmziqrs/gpui-query"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="mr-2 h-4 w-4" />
                GitHub
              </a>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ─── Features ─────────────────────────────────────────────────── */

const features = [
  {
    icon: Zap,
    title: "Declarative Queries",
    description:
      "Write queries declaratively. gpui-query handles fetching, caching, and state updates automatically.",
  },
  {
    icon: ArrowRightLeft,
    title: "Mutations",
    description:
      "First-class mutation support with success/error callbacks and optimistic updates.",
  },
  {
    icon: InfinityIcon,
    title: "Infinite Queries",
    description:
      "Paginate effortlessly with built-in infinite query support and bidirectional fetching.",
  },
  {
    icon: Database,
    title: "Smart Caching",
    description:
      "TTL, Stale-While-Revalidate, LatestWins, IgnoreWhileLoading — cache policies for every use case.",
  },
  {
    icon: XCircle,
    title: "Cancellation",
    description:
      "Signal-checked retries and cooperative cancellation via Arc<AtomicBool> for clean async lifecycle management.",
  },
  {
    icon: HardDrive,
    title: "Persistence",
    description: "Serialize and restore query state with custom persistence backends.",
  },
] as const;

function FeatureGrid() {
  return (
    <section className="border-t border-primary/5 bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Everything you need for async state
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-foreground/60">
            A complete toolkit for managing asynchronous data flows in your GPUI applications.
          </p>
        </div>

        <div className="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature) => (
            <Card
              key={feature.title}
              className="group border border-border bg-card shadow-md transition-all duration-200 hover:-translate-y-1 hover:border-primary/30 hover:shadow-xl hover:shadow-primary/10"
            >
              <CardHeader>
                <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-primary/12 text-primary transition-colors group-hover:bg-primary/20">
                  <feature.icon className="h-5 w-5" />
                </div>
                <h3 className="text-lg font-semibold">{feature.title}</h3>
              </CardHeader>
              <CardContent>
                <p className="text-sm leading-relaxed text-foreground/70">{feature.description}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ─── Quick Start ──────────────────────────────────────────────── */

function QuickStartSection() {
  return (
    <section className="py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl">
          <div className="border-l-4 border-primary pl-5">
            <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">Quick Start</h2>
            <p className="mt-2 text-muted-foreground">
              Start fetching data in just a few lines of Rust.
            </p>
          </div>

          <div className="mt-10 rounded-xl bg-muted/30 p-6">
            <div className="mb-4 flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Terminal className="h-4 w-4" />
              Install the crate
            </div>
            <CodeBlock title="Cargo.toml">
              <pre className="overflow-x-auto rounded-b-lg border border-t-0 bg-muted/50 p-4 text-sm leading-relaxed font-mono">
                <code>{`[dependencies]
gpui-query = "0.1"`}</code>
              </pre>
            </CodeBlock>

            <div className="mt-6 flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Code2 className="h-4 w-4" />
              Use it in your view
            </div>
            <div className="mt-3">
              <CodeBlock title="src/main.rs">
                <pre className="overflow-x-auto rounded-b-lg border border-t-0 bg-muted/50 p-4 text-sm leading-relaxed font-mono">
                  <code>{`use gpui_query::prelude::*;

fn render_user_list(cx: &mut ViewContext<App>) -> impl IntoElement {
    let query = use_query(cx, "user-list", || async {
        fetch_users().await
    });

    div().children(match &query.data {
        Some(users) => users.iter().map(|u| render_user(u)),
        None => vec![div().child("Loading...")],
    })
}`}</code>
                </pre>
              </CodeBlock>
            </div>
          </div>

          <div className="mt-8 text-center">
            <Button variant="outline" asChild>
              <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
                Read the full guide
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ─── Comparison Table ─────────────────────────────────────────── */

type Support = boolean;
type Row = [string, Support, Support, Support];

const comparisonRows: Row[] = [
  ["Caching", true, false, false],
  ["Auto Retry", true, false, false],
  ["Deduplication", true, false, false],
  ["Cache Policies", true, false, false],
  ["DevTools", true, false, false],
  ["Persistence", true, false, false],
  ["Type Safety", true, true, true],
];

function ComparisonTable() {
  return (
    <section className="border-t border-primary/5 bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <h2 className="text-center text-3xl font-bold tracking-tight sm:text-4xl">
          Why gpui-query?
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-foreground/60">
          See how gpui-query compares to the alternatives for managing async state in GPUI.
        </p>

        <div className="mx-auto mt-14 max-w-3xl overflow-hidden rounded-xl border border-border shadow-sm">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="bg-muted/80">
                <th className="py-4 pr-4 pl-5 text-left text-sm font-semibold text-foreground">
                  Feature
                </th>
                <th className="px-4 py-4 text-center">
                  <div className="flex flex-col items-center gap-2">
                    <span className="inline-flex items-center gap-1.5 rounded-full bg-primary/15 px-4 py-1.5 text-sm font-bold text-primary">
                      gpui-query
                    </span>
                    <span className="rounded-full bg-primary/20 px-2.5 py-0.5 text-[10px] font-bold uppercase tracking-wider text-primary">
                      Recommended
                    </span>
                  </div>
                </th>
                <th className="px-4 py-4 text-center text-sm font-semibold text-foreground/60">
                  Manual cx.spawn()
                </th>
                <th className="pl-4 pr-5 py-4 text-center text-sm font-semibold text-foreground/60">
                  Raw Futures
                </th>
              </tr>
            </thead>
            <tbody>
              {comparisonRows.map(([feature, a, b, c], i) => (
                <tr
                  key={feature}
                  className={`border-b border-border last:border-0 transition-colors hover:bg-muted/40 ${i % 2 === 0 ? "bg-background" : "bg-muted/30"}`}
                >
                  <td className="py-4 pr-4 pl-5 text-sm font-medium text-foreground">{feature}</td>
                  <td className="bg-primary/[0.04] px-4 py-4 text-center">
                    <SupportCell value={a} />
                  </td>
                  <td className="px-4 py-4 text-center">
                    <SupportCell value={b} />
                  </td>
                  <td className="pl-4 pr-5 py-4 text-center">
                    <SupportCell value={c} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}

function SupportCell({ value }: { value: boolean }) {
  if (value) {
    return (
      <span className="inline-flex h-7 w-7 items-center justify-center rounded-full bg-primary/20 text-primary">
        <Check className="h-4 w-4" strokeWidth={3} />
      </span>
    );
  }
  return (
    <span className="inline-flex h-7 w-7 items-center justify-center rounded-full bg-destructive/12 text-destructive/70">
      <X className="h-4 w-4" strokeWidth={3} />
    </span>
  );
}

/* ─── Architecture ─────────────────────────────────────────────── */

const archLayers = [
  {
    label: "Hook Layer",
    items: "use_query / use_mutation / use_infinite_query",
    icon: Layers,
    bgClass: "bg-primary/[0.08] border-2 border-primary/30 shadow-sm",
    textClass: "text-primary",
    iconBgClass: "bg-primary/20 text-primary",
  },
  {
    label: "Client Layer",
    items: "QueryClient / Registry / GC",
    icon: Cpu,
    bgClass: "bg-emerald-500/[0.08] border-2 border-emerald-500/30 shadow-sm",
    textClass: "text-emerald-600 dark:text-emerald-400",
    iconBgClass: "bg-emerald-500/20 text-emerald-600 dark:text-emerald-400",
  },
  {
    label: "Core Layer",
    items: "QueryResource / CachePolicy / QueryKey",
    icon: Database,
    bgClass: "bg-teal-500/[0.08] border-2 border-teal-500/30 shadow-sm",
    textClass: "text-teal-600 dark:text-teal-400",
    iconBgClass: "bg-teal-500/20 text-teal-600 dark:text-teal-400",
  },
];

function ArchitectureSection() {
  return (
    <section className="py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <h2 className="text-center text-3xl font-bold tracking-tight sm:text-4xl">
          Three-Layer Architecture
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-foreground/60">
          Designed in three clean layers, each with a clear responsibility.
        </p>

        <div className="mx-auto mt-16 max-w-3xl">
          <div className="grid gap-0">
            {archLayers.map((layer, i) => (
              <div key={layer.label}>
                {/* Connecting line */}
                {i > 0 && (
                  <div className="flex justify-center">
                    <div className="flex h-12 w-px flex-col items-center">
                      <div className="h-full w-px bg-gradient-to-b from-primary/40 to-primary/20" />
                      <div
                        className="h-0 w-0"
                        style={{
                          borderLeft: "6px solid transparent",
                          borderRight: "6px solid transparent",
                          borderTop: "7px solid var(--color-primary)",
                          opacity: 0.5,
                          marginTop: "-1px",
                        }}
                      />
                    </div>
                  </div>
                )}

                {/* Layer box */}
                <div
                  className={`flex items-center gap-4 rounded-xl px-6 py-5 ${layer.bgClass} transition-all duration-200 hover:scale-[1.02]`}
                >
                  <div
                    className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg ${layer.iconBgClass}`}
                  >
                    <layer.icon className="h-5 w-5" />
                  </div>
                  <div className="min-w-0">
                    <p className="text-xs font-bold uppercase tracking-widest opacity-80">
                      {layer.label}
                    </p>
                    <p className={`mt-0.5 truncate font-mono text-sm ${layer.textClass}`}>
                      {layer.items}
                    </p>
                  </div>
                </div>
              </div>
            ))}
          </div>

          {/* Flow arrows between layers */}
          <div className="mt-8 flex items-center justify-center gap-3 text-sm text-muted-foreground">
            <span className="font-medium">Data flows</span>
            <span className="text-primary">Hook</span>
            <ArrowRight className="h-4 w-4 text-muted-foreground" />
            <span className="text-emerald-600 dark:text-emerald-400">Client</span>
            <ArrowRight className="h-4 w-4 text-muted-foreground" />
            <span className="text-teal-600 dark:text-teal-400">Core</span>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ─── CTA Footer ───────────────────────────────────────────────── */

function CtaFooterSection() {
  return (
    <section className="border-t border-primary/5 py-20 sm:py-28">
      <div
        className="absolute inset-0 -z-10 opacity-[0.03]"
        aria-hidden="true"
        style={{
          backgroundImage: "radial-gradient(circle, var(--color-primary) 1px, transparent 1px)",
          backgroundSize: "20px 20px",
        }}
      />
      <div className="mx-auto max-w-7xl px-4 text-center sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl rounded-2xl bg-gradient-to-br from-primary/10 via-primary/5 to-emerald-500/10 px-8 py-16 sm:px-16">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Ready to simplify async state?
          </h2>
          <p className="mx-auto mt-4 max-w-lg text-muted-foreground">
            Get started with gpui-query in minutes and focus on building great applications, not
            managing async boilerplate.
          </p>

          <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Button
              size="lg"
              asChild
              className="px-6 text-base font-semibold shadow-lg shadow-primary/20"
            >
              <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
                Get Started
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>

            <Button variant="outline" size="lg" asChild className="px-6 text-base font-semibold">
              <a
                href="https://github.com/hmziqrs/gpui-query"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="mr-2 h-4 w-4" />
                View on GitHub
              </a>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}
