import { ArrowRightIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "#/components/ui/button";
import { CommandLine, CornerMarks } from "./decor";
import { RustCode } from "./rust-code";
import { VersionSwitcher } from "./version-switcher";

/* ══ V1 · SIGNAL PATH ═══════════════════════════════════════════════
   The page as a live wiring diagram: data visibly travels the route
   render → use_query → QueryClient → cache/network. Idle interactivity:
   cursor-reactive grid, self-running SWR cycle, operable schematic.  */

type CacheState = "fresh" | "stale";
type RouteName = "reqNet" | "reqCache" | "serveNet" | "serveCache" | "clientNet";

const PULSE_PATHS: Record<RouteName, { path: string; dur: number }> = {
  reqNet: { path: "M144,160 H584 V250 H628", dur: 1000 },
  reqCache: { path: "M144,160 H584 V70 H628", dur: 1000 },
  serveNet: { path: "M628,250 H584 V160 H144", dur: 1000 },
  serveCache: { path: "M628,70 H584 V160 H144", dur: 1000 },
  clientNet: { path: "M544,160 H584 V250 H628", dur: 600 },
};

const TRACES = [
  "M144,160 H204",
  "M336,160 H396",
  "M544,160 H584 V70 H628",
  "M544,160 H584 V250 H628",
];

interface LogLine {
  id: number;
  text: string;
}

function useSchematic() {
  const [cache, setCache] = useState<CacheState>("fresh");
  const [busy, setBusy] = useState(false);
  const [log, setLog] = useState<LogLine[]>([
    { id: 0, text: 'observer attached · watching "users"' },
  ]);
  const [flights, setFlights] = useState<Record<RouteName, boolean>>({
    reqNet: false,
    reqCache: false,
    serveNet: false,
    serveCache: false,
    clientNet: false,
  });

  const pulseRefs = useRef<Partial<Record<RouteName, SVGAnimationElement>>>({});
  const timeouts = useRef<number[]>([]);
  const busyRef = useRef(false);
  const reducedRef = useRef(false);
  const logId = useRef(0);

  useEffect(() => {
    reducedRef.current = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const saved = timeouts.current;
    return () => saved.forEach(clearTimeout);
  }, []);

  const after = useCallback((ms: number, fn: () => void) => {
    timeouts.current.push(window.setTimeout(fn, ms));
  }, []);

  const pushLog = useCallback((text: string) => {
    logId.current += 1;
    const id = logId.current;
    setLog((prev) => [{ id, text }, ...prev].slice(0, 3));
  }, []);

  const firePulse = useCallback((route: RouteName) => {
    if (reducedRef.current) return;
    const el = pulseRefs.current[route];
    if (!el) return;
    setFlights((f) => ({ ...f, [route]: true }));
    el.beginElement();
    timeouts.current.push(
      window.setTimeout(
        () => setFlights((f) => ({ ...f, [route]: false })),
        PULSE_PATHS[route].dur,
      ),
    );
  }, []);

  const run = useCallback(
    (steps: { at: number; go: () => void }[]) => {
      if (busyRef.current) return;
      busyRef.current = true;
      setBusy(true);
      const reduced = reducedRef.current;
      let total = 0;
      steps.forEach((s) => {
        total = Math.max(total, s.at);
        after(reduced ? 0 : s.at, s.go);
      });
      after(reduced ? 50 : total + 300, () => {
        busyRef.current = false;
        setBusy(false);
      });
    },
    [after],
  );

  const refetch = useCallback(() => {
    run([
      {
        at: 0,
        go: () => {
          pushLog('→ refetch("users") · forced');
          firePulse("reqNet");
        },
      },
      {
        at: 1000,
        go: () => {
          firePulse("serveNet");
          pushLog("← 200 OK · 142ms");
        },
      },
      {
        at: 2000,
        go: () => {
          setCache("fresh");
          pushLog("cache updated · observers notified");
        },
      },
    ]);
  }, [run, pushLog, firePulse]);

  const invalidate = useCallback(() => {
    run([
      {
        at: 0,
        go: () => {
          pushLog('→ invalidate_queries(Exact("users"))');
          firePulse("reqCache");
        },
      },
      {
        at: 1000,
        go: () => {
          setCache("stale");
          pushLog("entry marked stale");
        },
      },
      {
        at: 1600,
        go: () => {
          pushLog("refetching in background…");
          firePulse("clientNet");
        },
      },
      { at: 2200, go: () => firePulse("serveNet") },
      {
        at: 3200,
        go: () => {
          setCache("fresh");
          pushLog("← 200 OK · 89ms · fresh again");
        },
      },
    ]);
  }, [run, pushLog, firePulse]);

  // Idle: a stale-while-revalidate cycle plays on its own every ~14s.
  useEffect(() => {
    const id = window.setInterval(() => {
      if (busyRef.current || reducedRef.current || document.hidden) return;
      run([
        {
          at: 0,
          go: () => {
            setCache("stale");
            pushLog("ttl elapsed → stale");
          },
        },
        {
          at: 400,
          go: () => {
            firePulse("serveCache");
            pushLog("serving cached data · 0ms");
          },
        },
        {
          at: 1500,
          go: () => {
            firePulse("clientNet");
            pushLog("revalidating in background…");
          },
        },
        { at: 2100, go: () => firePulse("serveNet") },
        {
          at: 3100,
          go: () => {
            setCache("fresh");
            pushLog("revalidated · cache fresh");
          },
        },
      ]);
    }, 14000);
    return () => clearInterval(id);
  }, [run, pushLog, firePulse]);

  return { cache, busy, log, flights, pulseRefs, refetch, invalidate };
}

function SchematicNode({
  x,
  y,
  w,
  h,
  title,
  sub,
  active = false,
  dashed = false,
}: {
  x: number;
  y: number;
  w: number;
  h: number;
  title: string;
  sub?: string;
  active?: boolean;
  dashed?: boolean;
}) {
  return (
    <g>
      <rect
        x={x}
        y={y}
        width={w}
        height={h}
        className={`fill-card transition-[stroke] duration-300 ${
          active ? "stroke-primary/70" : "stroke-border"
        }`}
        strokeWidth={1}
        strokeDasharray={dashed ? "4 3" : undefined}
      />
      <text
        x={x + w / 2}
        y={y + (sub ? h / 2 - 2 : h / 2 + 4)}
        textAnchor="middle"
        className="fill-foreground font-mono"
        fontSize={12}
      >
        {title}
      </text>
      {sub && (
        <text
          x={x + w / 2}
          y={y + h / 2 + 13}
          textAnchor="middle"
          className="fill-muted-foreground font-mono"
          fontSize={9}
          letterSpacing={1}
        >
          {sub}
        </text>
      )}
    </g>
  );
}

export function SchematicPanel() {
  const { cache, busy, log, flights, pulseRefs, refetch, invalidate } = useSchematic();
  const netActive = flights.reqNet || flights.serveNet || flights.clientNet;
  const cacheActive = flights.reqCache || flights.serveCache;

  return (
    <div className="relative">
      <CornerMarks />
      <div className="border border-border bg-background/80 shadow-sm backdrop-blur-[2px]">
        <div className="flex items-center gap-2 border-b border-border px-4 py-2.5">
          <span className="font-mono text-xs text-muted-foreground">schematic://query-path</span>
          <span className="ml-auto flex items-center gap-2 font-mono text-[11px] tracking-widest text-primary">
            <span className={`h-1.5 w-1.5 bg-primary ${busy ? "animate-pulse" : ""}`} />
            LIVE
          </span>
        </div>

        <svg
          viewBox="0 0 760 320"
          role="img"
          aria-label="Diagram of a query traveling from your view through use_query and QueryClient to the cache and network"
          className="block w-full"
        >
          {TRACES.map((d) => (
            <path key={d} d={d} className="stroke-border" strokeWidth={1} fill="none" />
          ))}

          <SchematicNode x={24} y={136} w={120} h={48} title="render()" sub="YOUR VIEW" />
          <SchematicNode x={204} y={136} w={132} h={48} title="use_query" sub="HOOK" />
          <SchematicNode
            x={396}
            y={136}
            w={148}
            h={48}
            title="QueryClient"
            sub="GLOBAL"
            active={busy}
          />
          <SchematicNode
            x={628}
            y={48}
            w={108}
            h={44}
            title="CACHE"
            sub={cache === "fresh" ? "FRESH" : "STALE"}
            active={cacheActive || cache === "fresh"}
            dashed={cache === "stale"}
          />
          <SchematicNode
            x={628}
            y={228}
            w={108}
            h={44}
            title="NETWORK"
            sub="FETCHER"
            active={netActive}
          />

          {(Object.keys(PULSE_PATHS) as RouteName[]).map((route) => (
            <circle
              key={route}
              r={3.5}
              className={`fill-primary ${flights[route] ? "opacity-100" : "opacity-0"}`}
            >
              <animateMotion
                ref={(el) => {
                  pulseRefs.current[route] = (el as unknown as SVGAnimationElement) ?? undefined;
                }}
                begin="indefinite"
                dur={`${String(PULSE_PATHS[route].dur)}ms`}
                path={PULSE_PATHS[route].path}
                calcMode="linear"
              />
            </circle>
          ))}
        </svg>

        <div className="flex flex-col gap-3 border-t border-border px-4 py-3 sm:flex-row sm:items-center">
          <div className="flex gap-2">
            <button
              type="button"
              onClick={refetch}
              disabled={busy}
              className="border border-primary/40 bg-primary/5 px-3 py-1.5 font-mono text-xs text-primary transition-colors hover:bg-primary/15 focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none disabled:opacity-40"
            >
              refetch()
            </button>
            <button
              type="button"
              onClick={invalidate}
              disabled={busy}
              className="border border-border px-3 py-1.5 font-mono text-xs text-foreground transition-colors hover:border-primary/40 hover:text-primary focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none disabled:opacity-40"
            >
              invalidate()
            </button>
          </div>
          <p className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground sm:text-right">
            <span aria-hidden="true" className="text-primary/60">
              ▸{" "}
            </span>
            {log[0]?.text}
          </p>
        </div>
      </div>
    </div>
  );
}

/* ── Modules on the spine ─────────────────────────────────────────── */

const MODULES = [
  {
    api: "use_query",
    title: "Declarative queries",
    desc: "One hook wires fetch, cache, and re-render. The fetcher runs on the background executor; results land in an Entity your view reads.",
  },
  {
    api: "CachePolicy",
    title: "Caching, chosen per query",
    desc: "NoCache, Ttl, or StaleWhileRevalidate — declare it in QueryOptions and the client enforces it.",
  },
  {
    api: "RetryPolicy",
    title: "Retry with backoff",
    desc: "Max attempts, base delay, exponential backoff, capped. Cancelled queries stop retrying immediately.",
  },
  {
    api: "QuerySignal",
    title: "Cooperative cancellation",
    desc: "Every fetcher receives a signal. Drop the view and in-flight work stops cleanly — no orphaned tasks.",
  },
  {
    api: "use_mutation",
    title: "Mutations",
    desc: "A begin / complete / retry lifecycle with on_success and on_error callbacks, plus optimistic set and rollback.",
  },
  {
    api: "use_infinite_query",
    title: "Infinite queries",
    desc: "Bidirectional pagination with per-query page caps. The fetcher gets the last page and returns (items, has_more).",
  },
];

function ModuleLadder() {
  return (
    <section className="border-t border-border py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
          <span className="text-primary/50">{"// "}</span>modules
        </p>
        <h2 className="font-display mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
          Everything taps the same spine
        </h2>
        <p className="mt-4 max-w-2xl text-muted-foreground">
          Each capability is a component on the query path — named here by the actual API that
          provides it.
        </p>

        <div className="relative mt-14 pl-8 sm:pl-0">
          {/* Spine */}
          <div
            aria-hidden="true"
            className="absolute top-0 bottom-0 left-0 w-px bg-border sm:left-1/2"
          />
          <div className="grid grid-cols-1 gap-x-16 gap-y-10 sm:grid-cols-2">
            {MODULES.map((mod, i) => {
              const leftCol = i % 2 === 0;
              return (
                <div
                  key={mod.api}
                  className="group relative border border-border bg-card p-6 transition-colors duration-300 hover:border-primary/50"
                >
                  {/* Tap stub to the spine */}
                  <span
                    aria-hidden="true"
                    className={`absolute top-8 h-px w-8 bg-border transition-colors duration-300 group-hover:bg-primary ${
                      leftCol ? "-left-8 sm:left-auto sm:-right-8" : "-left-8"
                    }`}
                  />
                  <span
                    aria-hidden="true"
                    className={`absolute top-[29.5px] h-[5px] w-[5px] bg-border transition-colors duration-300 group-hover:bg-primary ${
                      leftCol ? "-left-[34px] sm:left-auto sm:-right-[34px]" : "-left-[34px]"
                    }`}
                  />
                  <span
                    aria-hidden="true"
                    className="v1-led absolute top-3 right-3 h-1.5 w-1.5 bg-primary opacity-0 transition-opacity group-hover:opacity-100"
                  />
                  <p className="font-mono text-xs text-primary">{mod.api}</p>
                  <h3 className="font-display mt-2 text-base font-semibold tracking-wide">
                    {mod.title}
                  </h3>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{mod.desc}</p>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}

/* ── The three hooks ──────────────────────────────────────────────── */

const HOOK_TABS = [
  {
    id: "use_query",
    returns: "(Entity<QueryResource<T, E>>, Subscription)",
    code: `let (users, _sub) = use_query(
    QueryOptions::new("users")
        .cache_policy(CachePolicy::StaleWhileRevalidate {
            ttl_ms: 60_000,
            stale_ms: 300_000,
        })
        .retry_policy(RetryPolicy::new(3).with_exponential_backoff()),
    |signal| async move { fetch_users(&signal).await },
    cx,
);`,
  },
  {
    id: "use_mutation",
    returns: "(Entity<MutationResource<V, T, E>>, Subscription)",
    code: `let (create, _sub) = use_mutation((), cx);

mutate_with_callbacks(
    &create,
    NewUser { name: "Alice" },
    |vars| async move { create_user(vars).await },
    MutationCallbacks::new()
        .on_success(|_| { /* invalidate "users" */ })
        .on_error(|err| eprintln!("failed: {err:?}")),
    cx,
);`,
  },
  {
    id: "use_infinite_query",
    returns: "(Entity<InfiniteQueryResource<P, E>>, Subscription)",
    code: `let (feed, _sub) = use_infinite_query(
    InfiniteQueryOptions::new(QueryKey::from(["feed"]))
        .max_pages(Some(10)),
    |last_page| async move {
        let cursor = last_page.map(|p| p.cursor());
        let page = fetch_page(cursor).await?;
        Ok((page.items, page.has_more))
    },
    cx,
);`,
  },
];

export function HookTabs() {
  const [active, setActive] = useState(HOOK_TABS[0].id);
  const tab = HOOK_TABS.find((t) => t.id === active) ?? HOOK_TABS[0];

  return (
    <section className="border-t border-border py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="grid items-start gap-12 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
          <div>
            <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
              <span className="text-primary/50">{"// "}</span>surface area
            </p>
            <h2 className="font-display mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
              Three hooks. That's the API.
            </h2>
            <p className="mt-4 text-muted-foreground">
              Queries, mutations, and pagination share the same shape: a key, an async function, and
              your context. Everything returns an Entity you read in render.
            </p>
            <div className="mt-8">
              <Button variant="outline" asChild>
                <a href="/docs/">
                  Read the full guide
                  <ArrowRightIcon size={14} className="ml-1" />
                </a>
              </Button>
            </div>
          </div>

          <div className="relative">
            <CornerMarks />
            <div className="border border-border bg-card shadow-sm">
              <div
                role="tablist"
                aria-label="Hook examples"
                className="flex border-b border-border"
              >
                {HOOK_TABS.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    role="tab"
                    aria-selected={t.id === active}
                    onClick={() => setActive(t.id)}
                    className={`border-r border-border px-4 py-3 font-mono text-xs transition-colors focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none ${
                      t.id === active
                        ? "bg-primary/10 text-primary"
                        : "text-muted-foreground hover:text-foreground"
                    }`}
                  >
                    {t.id}
                  </button>
                ))}
              </div>
              <RustCode code={tab.code} className="min-h-[290px]" />
              <div className="border-t border-border px-5 py-2.5 font-mono text-[11px] text-muted-foreground">
                <span className="text-primary/60">→ </span>
                {tab.returns}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ── Spec strip ───────────────────────────────────────────────────── */

const SPECS = [
  ["crate", "gpui-query"],
  ["version", "0.2.0"],
  ["license", "MIT"],
  ["layers", "core · client · hook"],
  ["default ttl", "60 s"],
  ["gc after", "5 min idle"],
];

function SpecStrip() {
  return (
    <section aria-label="Crate facts" className="border-t border-border">
      <div className="mx-auto grid max-w-7xl grid-cols-2 sm:grid-cols-3 lg:grid-cols-6">
        {SPECS.map(([label, value]) => (
          <div
            key={label}
            className="border-r border-b border-border px-4 py-4 transition-colors last:border-r-0 hover:bg-muted/40 sm:border-b-0"
          >
            <p className="font-mono text-[10px] tracking-[0.2em] uppercase text-muted-foreground">
              {label}
            </p>
            <p className="mt-1 font-mono text-sm text-foreground">{value}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

/* ── Hero + page ──────────────────────────────────────────────────── */

function HeroV1() {
  const glowRef = useRef<HTMLDivElement>(null);

  const handleMove = useCallback((e: React.MouseEvent<HTMLElement>) => {
    const el = glowRef.current;
    if (!el) return;
    const rect = e.currentTarget.getBoundingClientRect();
    el.style.setProperty("--mx", `${String(e.clientX - rect.left)}px`);
    el.style.setProperty("--my", `${String(e.clientY - rect.top)}px`);
  }, []);

  return (
    <section className="relative overflow-hidden" onMouseMove={handleMove}>
      <div aria-hidden="true" className="v1-grid-base pointer-events-none absolute inset-0 -z-10" />
      <div
        ref={glowRef}
        aria-hidden="true"
        className="v1-grid-glow pointer-events-none absolute inset-0 -z-10"
      />

      <div className="mx-auto max-w-7xl px-4 pt-16 pb-16 sm:px-6 sm:pt-24 sm:pb-20 lg:px-8">
        <div className="grid items-center gap-12 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-14">
          <div>
            <div className="inline-flex items-center gap-2.5 border border-primary/25 bg-primary/5 px-3 py-1.5 font-mono text-[11px] tracking-[0.2em] uppercase text-primary">
              <span className="h-1.5 w-1.5 bg-primary" />
              v0.2.0 · async state for GPUI
            </div>

            <h1 className="font-display mt-8 text-5xl font-bold tracking-tight text-foreground sm:text-6xl">
              One path for
              <br />
              every <span className="text-primary">fetch</span>.
            </h1>

            <p className="mt-6 max-w-lg text-lg leading-relaxed text-muted-foreground">
              use_query routes network, cache, and render for you. Caching, retry, dedup, and
              revalidation are the wiring — not your code. Built for{" "}
              <span className="text-foreground">GPUI</span>, the framework behind Zed.
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

            <p className="mt-8 font-mono text-xs text-muted-foreground">
              <span className="text-primary/60">▸ </span>
              the diagram on the right is running — try invalidate()
            </p>
          </div>

          <SchematicPanel />
        </div>
      </div>
    </section>
  );
}

function CtaV1() {
  return (
    <section className="relative overflow-hidden border-t border-border py-24 sm:py-28">
      <div
        aria-hidden="true"
        className="v1-grid-base pointer-events-none absolute inset-0 -z-10 [mask-image:radial-gradient(ellipse_70%_70%_at_50%_100%,black,transparent)]"
      />
      <div className="mx-auto max-w-2xl px-4 text-center sm:px-6">
        <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
          <span className="text-primary/50">{"// "}</span>close the loop
        </p>
        <h2 className="font-display mt-3 text-4xl font-bold tracking-tight sm:text-5xl">
          Wire it once
        </h2>
        <p className="mx-auto mt-4 max-w-md text-muted-foreground">
          One dependency between your views and hand-rolled async lifecycle code.
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

export function LandingV1() {
  return (
    <div className="flex flex-col">
      <HeroV1 />
      <ModuleLadder />
      <HookTabs />
      <SpecStrip />
      <CtaV1 />
      <VersionSwitcher current="v1" />
    </div>
  );
}
