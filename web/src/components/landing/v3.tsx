import { ArrowRightIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { useCallback, useRef, useState } from "react";
import { Button } from "#/components/ui/button";
import { CommandLine } from "./decor";
import { tintRust } from "./rust-tint";
import { VersionSwitcher } from "./version-switcher";

/* ══ V3 · BLUEPRINT ═════════════════════════════════════════════════
   The page as an engineering drawing: ruler-tick sheet frame, a live
   title block, a crosshair with a coordinate readout that follows the
   cursor, and annotated figures where hovering a callout highlights
   the exact line of API it explains.                                 */

/* ── Drafting primitives ──────────────────────────────────────────── */

function DimensionLine({ label }: { label: string }) {
  return (
    <div aria-hidden="true" className="flex items-center font-mono text-[10px] text-primary/80">
      <span className="h-3 w-px bg-primary/50" />
      <span className="h-px flex-1 bg-primary/40" />
      <span className="px-3 tracking-[0.25em] whitespace-nowrap uppercase">{label}</span>
      <span className="h-px flex-1 bg-primary/40" />
      <span className="h-3 w-px bg-primary/50" />
    </div>
  );
}

function FigCaption({ children }: { children: string }) {
  return (
    <p className="font-mono text-[11px] tracking-[0.2em] uppercase text-muted-foreground">
      <span className="text-primary">{children.split("—")[0]}</span>—{children.split("—")[1]}
    </p>
  );
}

function TitleBlock() {
  return (
    <div className="w-full max-w-xs border border-border bg-card font-mono text-[10px] shadow-sm">
      <div className="grid grid-cols-2 border-b border-border">
        <div className="border-r border-border px-3 py-2">
          <p className="tracking-[0.15em] text-muted-foreground">PROJECT</p>
          <p className="mt-0.5 text-xs text-foreground">gpui-query</p>
        </div>
        <div className="px-3 py-2">
          <p className="tracking-[0.15em] text-muted-foreground">DRAWING</p>
          <p className="mt-0.5 text-xs text-foreground">async state</p>
        </div>
      </div>
      <div className="grid grid-cols-4 border-b border-border">
        {[
          ["REV", "0.2.0"],
          ["LIC", "MIT"],
          ["SCALE", "1:1"],
          ["SHEET", "1/1"],
        ].map(([label, value]) => (
          <div key={label} className="border-r border-border px-2 py-1.5 last:border-r-0">
            <p className="tracking-[0.15em] text-muted-foreground">{label}</p>
            <p className="mt-0.5 text-xs text-foreground">{value}</p>
          </div>
        ))}
      </div>
      <div className="flex items-center justify-between px-3 py-1.5">
        <span className="tracking-[0.15em] text-muted-foreground">DRAWN BY</span>
        <span className="text-xs text-foreground">
          use_query
          <span aria-hidden="true" className="animate-cursor-blink text-primary">
            ▍
          </span>
        </span>
      </div>
    </div>
  );
}

/* ── Hero with crosshair readout ──────────────────────────────────── */

function HeroV3() {
  const hLine = useRef<HTMLDivElement>(null);
  const vLine = useRef<HTMLDivElement>(null);
  const readout = useRef<HTMLDivElement>(null);

  const handleMove = useCallback((e: React.MouseEvent<HTMLElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = Math.round(e.clientX - rect.left);
    const y = Math.round(e.clientY - rect.top);
    if (hLine.current) {
      hLine.current.style.transform = `translateY(${String(y)}px)`;
      hLine.current.style.opacity = "1";
    }
    if (vLine.current) {
      vLine.current.style.transform = `translateX(${String(x)}px)`;
      vLine.current.style.opacity = "1";
    }
    if (readout.current) {
      const flipX = x > rect.width - 180;
      const flipY = y > rect.height - 56;
      readout.current.style.transform = `translate(${String(x + (flipX ? -166 : 14))}px, ${String(
        y + (flipY ? -34 : 14),
      )}px)`;
      readout.current.style.opacity = "1";
      readout.current.textContent = `x ${String(x).padStart(4, "0")} · y ${String(y).padStart(4, "0")} · grid 32`;
    }
  }, []);

  const handleLeave = useCallback(() => {
    for (const ref of [hLine, vLine, readout]) {
      if (ref.current) ref.current.style.opacity = "0";
    }
  }, []);

  return (
    <section
      className="relative overflow-hidden cursor-crosshair"
      onMouseMove={handleMove}
      onMouseLeave={handleLeave}
    >
      <div
        aria-hidden="true"
        className="v1-grid-base pointer-events-none absolute inset-0 [mask-image:radial-gradient(ellipse_80%_80%_at_50%_0%,black,transparent)]"
      />

      {/* Crosshair */}
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 z-10">
        <div
          ref={hLine}
          className="absolute inset-x-0 top-0 h-px bg-primary/35 opacity-0 transition-opacity duration-200"
        />
        <div
          ref={vLine}
          className="absolute inset-y-0 left-0 w-px bg-primary/35 opacity-0 transition-opacity duration-200"
        />
        <div
          ref={readout}
          className="absolute top-0 left-0 border border-primary/30 bg-background/90 px-2 py-1 font-mono text-[10px] whitespace-nowrap text-primary opacity-0 transition-opacity duration-200"
        />
      </div>

      <div className="relative mx-auto max-w-7xl px-4 pt-16 pb-14 sm:px-6 sm:pt-24 sm:pb-16 lg:px-8">
        <div className="grid items-end gap-12 lg:grid-cols-[minmax(0,1fr)_auto]">
          <div>
            <div className="inline-flex items-center gap-2.5 border border-primary/25 bg-primary/5 px-3 py-1.5 font-mono text-[11px] tracking-[0.2em] uppercase text-primary">
              <span className="h-1.5 w-1.5 bg-primary" />
              DWG NO. GQ-002 · async state for GPUI
            </div>

            <h1 className="font-display mt-8 text-5xl font-bold tracking-tight text-foreground sm:text-6xl">
              The whole API
              <br />
              is <span className="text-primary">one call</span>.
            </h1>

            <div className="mt-8 inline-block max-w-full">
              <pre className="overflow-x-auto border border-border bg-card px-5 py-4 font-mono text-[13px] leading-6 text-foreground/85 shadow-sm">
                <code>
                  {tintRust('let (users, sub) = use_query("users", fetcher, cx);', "hero")}
                </code>
              </pre>
              <div className="mt-2">
                <DimensionLine label="everything else is defaults" />
              </div>
            </div>

            <p className="mt-7 max-w-lg text-lg leading-relaxed text-muted-foreground">
              Caching, retry, deduplication, revalidation, and garbage collection are behavior you
              tune — not code you write. Drawn for <span className="text-foreground">GPUI</span>,
              the framework behind Zed.
            </p>

            <div className="mt-10 flex flex-col gap-4 sm:flex-row">
              <Button size="lg" asChild>
                <a href="/docs/">
                  Get Started
                  <ArrowRightIcon size={16} className="ml-1" />
                </a>
              </Button>
              <Button variant="outline" size="lg" asChild>
                <a
                  href="https://github.com/freeoxide/gpui-query"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <GithubLogoIcon size={18} className="mr-1.5" />
                  GitHub
                </a>
              </Button>
            </div>
          </div>

          <div className="hidden lg:block">
            <TitleBlock />
          </div>
        </div>
      </div>
    </section>
  );
}

/* ── Fig. 1 — anatomy of a query ──────────────────────────────────── */

const FIG_LINES: string[] = [
  "let (users, _sub) = use_query(",
  '    QueryOptions::new("users")',
  "        .cache_policy(CachePolicy::StaleWhileRevalidate {",
  "            ttl_ms: 60_000,",
  "            stale_ms: 300_000,",
  "        })",
  "        .retry_policy(RetryPolicy::new(3)",
  "            .with_exponential_backoff()),",
  "    |signal| async move {",
  "        fetch_users(&signal).await",
  "    },",
  "    cx,",
  ");",
];

const REGIONS: Record<string, number[]> = {
  A: [1],
  B: [2, 3, 4, 5],
  C: [6, 7],
  D: [8, 9, 10],
  E: [11],
};

const MARK_AT: Record<number, string> = { 1: "A", 2: "B", 6: "C", 8: "D", 11: "E" };

const CALLOUTS = [
  {
    mark: "A",
    title: "the key",
    desc: 'Cache address and dedup identity. Every use_query("users") anywhere in the app shares this one entry and its in-flight request.',
  },
  {
    mark: "B",
    title: "the policy",
    desc: "Past ttl_ms, cached data is served instantly while a background refetch runs — for up to stale_ms.",
  },
  {
    mark: "C",
    title: "the retry",
    desc: "Three attempts with exponential backoff, capped. A cancelled query stops retrying immediately.",
  },
  {
    mark: "D",
    title: "the fetcher",
    desc: "Runs on the background executor. The QuerySignal makes cancellation cooperative — drop the view, the work stops.",
  },
  {
    mark: "E",
    title: "the context",
    desc: "Results land back on the main thread as an Entity + Subscription your view reads in render.",
  },
];

export function AnnotatedFigure() {
  const [active, setActive] = useState<string | null>(null);

  return (
    <section className="border-t border-border py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <FigCaption>FIG. 1 — anatomy of a query · hover a callout</FigCaption>
        <h2 className="font-display mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
          Every line, accounted for
        </h2>

        <div className="mt-12 grid gap-10 lg:grid-cols-[minmax(0,7fr)_minmax(0,5fr)]">
          {/* Code figure */}
          <div className="border border-border bg-card shadow-sm">
            <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
              <span className="font-mono text-xs text-muted-foreground">src/views/users.rs</span>
              <span className="font-mono text-[10px] tracking-[0.2em] uppercase text-primary/70">
                rust
              </span>
            </div>
            <pre className="overflow-x-auto py-4 font-mono text-[13px] leading-6 text-foreground/85">
              <code>
                {FIG_LINES.map((line, i) => {
                  const region = Object.entries(REGIONS).find(([, idx]) => idx.includes(i))?.[0];
                  const isActive = region !== undefined && region === active;
                  const mark = MARK_AT[i];
                  return (
                    <span
                      key={line}
                      onMouseEnter={() => setActive(region ?? null)}
                      onMouseLeave={() => setActive(null)}
                      className={`flex border-l-2 px-4 transition-colors duration-150 ${
                        isActive ? "border-l-primary bg-primary/10" : "border-l-transparent"
                      }`}
                    >
                      <span className="min-w-0 flex-1 whitespace-pre">
                        {tintRust(line, `f-${String(i)}`)}
                      </span>
                      {mark && (
                        <span
                          className={`ml-4 shrink-0 font-mono text-[11px] transition-colors ${
                            isActive ? "text-primary" : "text-muted-foreground/50"
                          }`}
                        >
                          ⟨{mark}⟩
                        </span>
                      )}
                    </span>
                  );
                })}
              </code>
            </pre>
          </div>

          {/* Callouts */}
          <ul className="space-y-2">
            {CALLOUTS.map((c) => {
              const isActive = active === c.mark;
              return (
                <li key={c.mark}>
                  <button
                    type="button"
                    onMouseEnter={() => setActive(c.mark)}
                    onMouseLeave={() => setActive(null)}
                    onFocus={() => setActive(c.mark)}
                    onBlur={() => setActive(null)}
                    className={`w-full border border-l-2 p-4 text-left transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none ${
                      isActive
                        ? "border-border border-l-primary bg-primary/5"
                        : "border-transparent border-l-border"
                    }`}
                  >
                    <span className="flex items-center gap-3">
                      <span
                        className={`flex h-5 w-5 items-center justify-center border font-mono text-[11px] transition-colors ${
                          isActive
                            ? "border-primary bg-primary text-primary-foreground"
                            : "border-border text-muted-foreground"
                        }`}
                      >
                        {c.mark}
                      </span>
                      <span className="font-mono text-sm text-foreground">{c.title}</span>
                    </span>
                    <span className="mt-2 block text-sm leading-relaxed text-muted-foreground">
                      {c.desc}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      </div>
    </section>
  );
}

/* ── Fig. 2 — exploded view ───────────────────────────────────────── */

const LAYERS = [
  {
    name: "hook",
    items: "use_query · use_mutation · use_infinite_query",
    note: 'feature = "hook"',
  },
  {
    name: "client",
    items: "QueryClient · QueryBucket · GC · invalidation",
    note: 'feature = "client" · default',
  },
  {
    name: "core",
    items: "QueryResource · CachePolicy · RetryPolicy · QueryKey",
    note: 'feature = "core" · zero GPUI deps',
  },
];

function ExplodedView() {
  return (
    <section className="border-t border-border bg-muted/30 py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <FigCaption>FIG. 2 — exploded view · one crate, three feature gates</FigCaption>
        <h2 className="font-display mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
          Take only the layers you need
        </h2>
        <p className="mt-4 max-w-2xl text-muted-foreground">
          The core state machine has no framework coupling at all — use it in any Rust project. The
          client and hooks bolt GPUI on top.
        </p>

        <div className="mt-14 max-w-2xl">
          {LAYERS.map((layer, i) => (
            <div key={layer.name}>
              {i > 0 && (
                <div
                  aria-hidden="true"
                  className="ml-24 h-7 w-px border-l border-dashed border-border"
                  style={{ marginLeft: `${String(i * 32 + 64)}px` }}
                />
              )}
              <div
                className="group border border-border bg-card px-6 py-4 shadow-sm transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/60"
                style={{
                  marginLeft: `${String(i * 32)}px`,
                  transform: "skewX(-12deg)",
                }}
              >
                <div
                  style={{ transform: "skewX(12deg)" }}
                  className="flex flex-wrap items-baseline gap-x-5 gap-y-1"
                >
                  <p className="font-display w-16 text-sm font-bold tracking-[0.2em] uppercase text-foreground group-hover:text-primary">
                    {layer.name}
                  </p>
                  <p className="min-w-0 flex-1 font-mono text-xs leading-5 text-muted-foreground">
                    {layer.items}
                  </p>
                  <p className="font-mono text-[10px] tracking-wide text-primary/70">
                    {layer.note}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Index of details ─────────────────────────────────────────────── */

const DETAILS: [string, string][] = [
  ["use_query_select", "Project cached data through a transform — no extra fetch."],
  ["QueryPersister", "dehydrate() and hydrate() query state across app launches."],
  ["QueryKeyFilter", "Invalidate by Exact, Prefix, or All."],
  ["QueryObserver", "Notifies your view only when status actually changes."],
  ["QueryError::sanitized()", "Redacts tokens, paths, and emails from error output."],
  ["PreparedFetch", "Imperative one-shot fetches you complete manually."],
];

function DetailIndex() {
  return (
    <section className="border-t border-border py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <FigCaption>TABLE 1 — index of details</FigCaption>
        <h2 className="font-display mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
          The fine print is also engineered
        </h2>

        <div className="mt-12 grid gap-px border border-border bg-border sm:grid-cols-2 lg:grid-cols-3">
          {DETAILS.map(([name, desc]) => (
            <div
              key={name}
              className="group bg-background p-5 transition-colors duration-200 hover:bg-card"
            >
              <p className="flex items-center gap-2 font-mono text-[13px] text-foreground group-hover:text-primary">
                <span
                  aria-hidden="true"
                  className="text-primary opacity-0 transition-opacity group-hover:opacity-100"
                >
                  →
                </span>
                <span className="-ml-4 transition-transform duration-200 group-hover:ml-0">
                  {name}
                </span>
              </p>
              <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{desc}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Stamp CTA ────────────────────────────────────────────────────── */

function CtaV3() {
  return (
    <section className="border-t border-border py-24 sm:py-28">
      <div className="mx-auto max-w-2xl px-4 text-center sm:px-6">
        <div className="inline-block -rotate-3 border border-primary/70 p-1.5 transition-transform duration-300 hover:rotate-0">
          <div className="border border-primary/70 px-8 py-5 font-mono uppercase">
            <p className="text-lg font-medium tracking-[0.3em] text-primary">Approved</p>
            <p className="mt-1 text-[10px] tracking-[0.25em] text-primary/70">
              for production async state
            </p>
          </div>
        </div>

        <h2 className="font-display mt-10 text-4xl font-bold tracking-tight sm:text-5xl">
          Sign off on the drawing
        </h2>
        <p className="mx-auto mt-4 max-w-md text-muted-foreground">
          One dependency, three hooks, zero hand-rolled lifecycle code.
        </p>
        <div className="mt-10 flex justify-center">
          <CommandLine command="cargo add gpui-query" />
        </div>
        <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <Button size="lg" asChild>
            <a href="/docs/">
              Get Started
              <ArrowRightIcon size={16} className="ml-1" />
            </a>
          </Button>
          <Button variant="outline" size="lg" asChild>
            <a
              href="https://github.com/freeoxide/gpui-query"
              target="_blank"
              rel="noopener noreferrer"
            >
              <GithubLogoIcon size={18} className="mr-1.5" />
              Star on GitHub
            </a>
          </Button>
        </div>
      </div>
    </section>
  );
}

/* ── Sheet frame + page ───────────────────────────────────────────── */

export function LandingV3() {
  return (
    <div className="px-3 py-4 sm:px-6 sm:py-6">
      <div className="relative border border-border">
        {/* Ruler strips */}
        <div
          aria-hidden="true"
          className="v3-ruler-x absolute inset-x-2.5 top-0 h-2.5 border-b border-border"
        />
        <div
          aria-hidden="true"
          className="v3-ruler-y absolute inset-y-2.5 left-0 hidden w-2.5 border-r border-border sm:block"
        />
        <div
          aria-hidden="true"
          className="absolute top-0 left-0 hidden h-2.5 w-2.5 border-r border-b border-border sm:block"
        />

        <div className="flex flex-col pt-2.5 sm:pl-2.5">
          <HeroV3 />
          <AnnotatedFigure />
          <ExplodedView />
          <DetailIndex />
          <CtaV3 />
        </div>
      </div>
      <VersionSwitcher current="v3" />
    </div>
  );
}
