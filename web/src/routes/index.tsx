import { createFileRoute, Link } from "@tanstack/react-router";
import {
  Search,
  RefreshCw,
  List,
  Database,
  Zap,
  HardDrive,
  ArrowRight,
  Github,
  Check,
  X,
} from "lucide-react";
import { Button } from "#/components/ui/button";
import { Card, CardHeader, CardContent } from "#/components/ui/card";
import { CodeBlock } from "#/components/code-block";

export const Route = createFileRoute("/")({ component: Home });

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
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 -z-10">
        <div className="absolute left-1/2 top-0 h-[600px] w-[800px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-gradient-to-br from-primary/20 via-primary/5 to-transparent blur-3xl animate-pulse [animation-duration:6s]" />
        <div className="absolute right-0 bottom-0 h-[400px] w-[400px] translate-x-1/3 translate-y-1/3 rounded-full bg-gradient-to-tl from-primary/10 to-transparent blur-2xl animate-pulse [animation-duration:8s]" />
      </div>

      <div className="mx-auto max-w-7xl px-4 pb-24 pt-20 sm:px-6 sm:pb-32 sm:pt-28 lg:px-8">
        <div className="mx-auto max-w-3xl text-center">
          <h1 className="text-4xl font-extrabold tracking-tight sm:text-5xl lg:text-6xl">
            <span className="bg-gradient-to-r from-foreground via-foreground to-muted-foreground bg-clip-text text-transparent">
              gpui-query
            </span>
          </h1>

          <p className="mt-4 text-xl font-medium text-muted-foreground sm:text-2xl">
            Zero-boilerplate async state management for GPUI
          </p>

          <p className="mx-auto mt-6 max-w-2xl text-base text-muted-foreground sm:text-lg">
            gpui-query brings the reactive query patterns you love from the web world into Rust GPUI
            applications. Fetch, cache, and synchronize async data with a single hook — no manual
            lifecycle management required.
          </p>

          <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Button size="lg" asChild>
              <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
                Get Started
                <ArrowRight className="ml-1 h-4 w-4" />
              </Link>
            </Button>

            <Button variant="outline" size="lg" asChild>
              <a
                href="https://github.com/hmziqrs/gpui-query"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="mr-1 h-4 w-4" />
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
    icon: Search,
    title: "Declarative Queries",
    description:
      "Write queries declaratively. gpui-query handles fetching, caching, and state updates automatically.",
  },
  {
    icon: RefreshCw,
    title: "Mutations",
    description:
      "First-class mutation support with success/error callbacks and optimistic updates.",
  },
  {
    icon: List,
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
    icon: Zap,
    title: "Cooperative Cancellation",
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
    <section className="border-t bg-muted/30 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <h2 className="text-center text-3xl font-bold tracking-tight sm:text-4xl">
          Everything you need for async state
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-muted-foreground">
          A complete toolkit for managing asynchronous data flows in your GPUI applications.
        </p>

        <div className="mt-14 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature) => (
            <Card key={feature.title} className="group transition-shadow hover:shadow-md">
              <CardHeader>
                <div className="mb-2 flex h-10 w-10 items-center justify-center rounded-md bg-primary/10 text-primary transition-colors group-hover:bg-primary/20">
                  <feature.icon className="h-5 w-5" />
                </div>
                <h3 className="text-lg font-semibold">{feature.title}</h3>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {feature.description}
                </p>
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
    <section className="py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl">
          <h2 className="text-center text-3xl font-bold tracking-tight sm:text-4xl">Quick Start</h2>
          <p className="mt-4 text-center text-muted-foreground">
            Start fetching data in just a few lines of Rust.
          </p>

          <div className="mt-10">
            <CodeBlock title="src/main.rs">
              <pre className="overflow-x-auto rounded-b-lg border border-t-0 bg-muted/50 p-4 text-sm font-mono">
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

          <div className="mt-8 text-center">
            <Button variant="outline" asChild>
              <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
                Read the full guide
                <ArrowRight className="ml-1 h-4 w-4" />
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
    <section className="border-t bg-muted/30 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <h2 className="text-center text-3xl font-bold tracking-tight sm:text-4xl">
          Why gpui-query?
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-muted-foreground">
          See how gpui-query compares to the alternatives for managing async state in GPUI.
        </p>

        <div className="mx-auto mt-12 max-w-3xl overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b">
                <th className="py-3 pr-4 text-left font-semibold">Feature</th>
                <th className="px-4 py-3 text-center font-semibold">
                  <span className="rounded-full bg-primary/10 px-3 py-1 text-primary">
                    gpui-query
                  </span>
                </th>
                <th className="px-4 py-3 text-center font-semibold text-muted-foreground">
                  Manual cx.spawn()
                </th>
                <th className="pl-4 py-3 text-center font-semibold text-muted-foreground">
                  Raw Futures
                </th>
              </tr>
            </thead>
            <tbody>
              {comparisonRows.map(([feature, a, b, c]) => (
                <tr key={feature} className="border-b last:border-0">
                  <td className="py-3 pr-4 font-medium">{feature}</td>
                  <td className="px-4 py-3 text-center">
                    <SupportCell value={a} />
                  </td>
                  <td className="px-4 py-3 text-center">
                    <SupportCell value={b} />
                  </td>
                  <td className="pl-4 py-3 text-center">
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
      <span className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-primary">
        <Check className="h-3.5 w-3.5" />
      </span>
    );
  }
  return (
    <span className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-muted text-muted-foreground">
      <X className="h-3.5 w-3.5" />
    </span>
  );
}

/* ─── Architecture ─────────────────────────────────────────────── */

function ArchitectureSection() {
  return (
    <section className="py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <h2 className="text-center text-3xl font-bold tracking-tight sm:text-4xl">
          Layered Architecture
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-muted-foreground">
          Designed in three clean layers, each with a clear responsibility.
        </p>

        <div className="mx-auto mt-14 flex max-w-4xl flex-col items-center gap-4">
          <ArchLayer
            label="Hook Layer"
            items="use_query / use_mutation / use_infinite_query"
            accent="bg-primary text-primary-foreground"
          />

          <ArchArrow />

          <ArchLayer
            label="Client Layer"
            items="QueryClient / Registry / GC"
            accent="bg-secondary text-secondary-foreground border"
          />

          <ArchArrow />

          <ArchLayer
            label="Core Layer"
            items="QueryResource / CachePolicy / QueryKey"
            accent="bg-muted text-foreground border"
          />
        </div>
      </div>
    </section>
  );
}

function ArchLayer({ label, items, accent }: { label: string; items: string; accent: string }) {
  return (
    <div className={`w-full rounded-lg px-6 py-5 text-center shadow-sm ${accent}`}>
      <p className="text-xs font-semibold uppercase tracking-widest opacity-70">{label}</p>
      <p className="mt-1 font-mono text-sm">{items}</p>
    </div>
  );
}

function ArchArrow() {
  return (
    <div className="flex h-8 items-center justify-center text-muted-foreground">
      <svg width="24" height="32" viewBox="0 0 24 32" fill="none" aria-hidden="true">
        <path
          d="M12 0v26m-6-6 6 6 6-6"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
}

/* ─── CTA Footer ───────────────────────────────────────────────── */

function CtaFooterSection() {
  return (
    <section className="border-t bg-muted/30 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 text-center sm:px-6 lg:px-8">
        <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
          Ready to simplify async state in GPUI?
        </h2>
        <p className="mx-auto mt-4 max-w-xl text-muted-foreground">
          Get started with gpui-query in minutes and focus on building great applications, not
          managing async boilerplate.
        </p>

        <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <Button size="lg" asChild>
            <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
              Get Started
              <ArrowRight className="ml-1 h-4 w-4" />
            </Link>
          </Button>

          <Button variant="outline" size="lg" asChild>
            <a
              href="https://github.com/hmziqrs/gpui-query"
              target="_blank"
              rel="noopener noreferrer"
            >
              <Github className="mr-1 h-4 w-4" />
              Star on GitHub
            </a>
          </Button>
        </div>
      </div>
    </section>
  );
}
