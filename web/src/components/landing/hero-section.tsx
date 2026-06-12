import { ArrowRightIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { Button } from "#/components/ui/button";
import { CornerMarks } from "./decor";

/* ── Live query lifecycle ─────────────────────────────────────────── */

type LifecycleState = "FETCHING" | "FRESH" | "STALE" | "REFETCH";

interface Phase {
  state: LifecycleState;
  detail: string;
  busy: boolean;
  ms: number;
}

const PHASES: Phase[] = [
  { state: "FETCHING", detail: "GET /users · in flight", busy: true, ms: 1800 },
  { state: "FRESH", detail: "24 rows · 142ms · cached", busy: false, ms: 2600 },
  { state: "STALE", detail: "ttl elapsed · serving cached data", busy: false, ms: 2000 },
  { state: "REFETCH", detail: "revalidating in background", busy: true, ms: 1800 },
  { state: "FRESH", detail: "24 rows · 89ms · deduped", busy: false, ms: 3000 },
];

const PIPS: LifecycleState[] = ["FETCHING", "FRESH", "STALE", "REFETCH"];

const stateColor: Record<LifecycleState, string> = {
  FETCHING: "text-foreground",
  FRESH: "text-primary",
  STALE: "text-muted-foreground",
  REFETCH: "text-foreground",
};

function useQueryLifecycle() {
  const [index, setIndex] = useState(0);

  useEffect(() => {
    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReduced) {
      setIndex(1); // settle on FRESH
      return;
    }
    const timer = setTimeout(() => setIndex((i) => (i + 1) % PHASES.length), PHASES[index].ms);
    return () => clearTimeout(timer);
  }, [index]);

  return PHASES[index];
}

/* ── Syntax-tinted hero snippet ───────────────────────────────────── */

const kw = "text-primary";
const str = "text-primary/60";
const dim = "text-muted-foreground";

function HeroCode() {
  return (
    <pre className="overflow-x-auto p-5 font-mono text-[13px] leading-6 text-foreground/85">
      <code>
        <span className={kw}>use</span> gpui_query::prelude::*;{"\n\n"}
        <span className={kw}>let</span> users = use_query(cx, <span className={str}>"users"</span>,
        || <span className={kw}>async</span> {"{"}
        {"\n"}
        {"    "}fetch_users().<span className={kw}>await</span>
        {"\n"}
        {"}"});{"\n\n"}
        <span className={kw}>match</span> &users.data {"{"}
        {"\n"}
        {"    "}
        <span className={kw}>Some</span>(list) {"=>"} render_list(list),{"\n"}
        {"    "}
        <span className={kw}>None</span> {"=>"} render_loading(),{"\n"}
        {"}"} <span className={dim}>{"// caching, retry, dedup — handled"}</span>
      </code>
    </pre>
  );
}

/* ── Terminal panel ───────────────────────────────────────────────── */

function QueryTerminal() {
  const phase = useQueryLifecycle();
  const phaseKey = `${phase.state}-${phase.detail}`;

  return (
    <div className="relative">
      <CornerMarks />
      <div className="border border-border bg-card shadow-sm">
        {/* Header */}
        <div className="flex items-center gap-2 border-b border-border px-4 py-3">
          <div className="flex gap-1.5" aria-hidden="true">
            <span className="h-2.5 w-2.5 bg-foreground/10" />
            <span className="h-2.5 w-2.5 bg-foreground/10" />
            <span className="h-2.5 w-2.5 bg-foreground/10" />
          </div>
          <span className="ml-2 font-mono text-xs text-muted-foreground">query://users</span>
          <span
            className={`ml-auto flex items-center gap-2 font-mono text-[11px] tracking-widest ${stateColor[phase.state]}`}
          >
            <span
              className={`h-1.5 w-1.5 ${phase.busy ? "animate-pulse bg-current" : "bg-primary"}`}
            />
            {phase.state}
          </span>
        </div>

        {/* Code */}
        <HeroCode />

        {/* Lifecycle track */}
        <div className="border-t border-border px-4 py-3">
          <div className="flex items-center gap-2 font-mono text-[10px] tracking-[0.15em]">
            {PIPS.map((pip, i) => (
              <span key={pip} className="flex items-center gap-2">
                {i > 0 && (
                  <span aria-hidden="true" className="text-muted-foreground/40">
                    →
                  </span>
                )}
                <span className={pip === phase.state ? "text-primary" : "text-muted-foreground/50"}>
                  {pip}
                </span>
              </span>
            ))}
          </div>
          <p
            key={phaseKey}
            className="animate-status-in mt-2 font-mono text-xs text-muted-foreground"
          >
            <span aria-hidden="true" className="text-primary/60">
              ▸{" "}
            </span>
            {phase.detail}
            {phase.busy && (
              <span aria-hidden="true" className="animate-cursor-blink">
                {" "}
                ▍
              </span>
            )}
          </p>
        </div>
      </div>
    </div>
  );
}

/* ── Capability ticker ────────────────────────────────────────────── */

const tickerItems = [
  "Smart caching",
  "Auto retry",
  "Request deduplication",
  "Optimistic mutations",
  "Infinite queries",
  "Cooperative cancellation",
  "Persistence",
  "Type-safe keys",
];

function CapabilityTicker() {
  const row = (hidden: boolean) => (
    <div
      aria-hidden={hidden}
      className="flex shrink-0 items-center gap-8 pr-8 font-mono text-[11px] tracking-[0.25em] uppercase text-muted-foreground"
    >
      {tickerItems.map((item) => (
        <span key={item} className="flex items-center gap-8">
          <span aria-hidden="true" className="text-primary/50">
            +
          </span>
          {item}
        </span>
      ))}
    </div>
  );

  return (
    <div className="overflow-hidden border-y border-border py-3.5">
      <div className="animate-marquee flex w-max">
        {row(false)}
        {row(true)}
      </div>
    </div>
  );
}

/* ── Hero ─────────────────────────────────────────────────────────── */

export function HeroSection() {
  return (
    <section className="relative overflow-hidden">
      {/* Dot grid, fading out toward the fold */}
      <div
        aria-hidden="true"
        className="bg-dot-grid pointer-events-none absolute inset-0 -z-10 [mask-image:radial-gradient(ellipse_75%_65%_at_50%_0%,black,transparent)]"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -top-40 -left-40 -z-10 h-[600px] w-[600px] bg-primary/5 blur-[120px]"
      />

      <div className="mx-auto max-w-7xl px-4 pt-20 pb-20 sm:px-6 sm:pt-28 sm:pb-24 lg:px-8">
        <div className="grid items-center gap-14 lg:grid-cols-2 lg:gap-16">
          {/* Left column */}
          <div>
            <div className="inline-flex items-center gap-2.5 border border-primary/25 bg-primary/5 px-3 py-1.5 font-mono text-[11px] tracking-[0.2em] uppercase text-primary">
              <span className="h-1.5 w-1.5 bg-primary" />
              Inspired by TanStack Query
            </div>

            <h1 className="font-display mt-8 text-5xl font-bold tracking-tight text-foreground sm:text-6xl lg:text-7xl">
              Async state,
              <br />
              <span className="text-primary">solved</span>
              <span
                aria-hidden="true"
                className="animate-cursor-blink ml-1 inline-block h-[0.85em] w-[0.45em] translate-y-[0.08em] bg-primary/80 align-baseline"
              />
            </h1>

            <p className="mt-6 max-w-lg text-lg leading-relaxed text-muted-foreground">
              gpui-query brings TanStack Query's caching, retries, and background revalidation to
              Rust — purpose-built for <span className="text-foreground">GPUI</span>, the framework
              behind Zed.
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
                  href="https://github.com/hmziqrs/gpui-query"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <GithubLogoIcon size={18} className="mr-1.5" />
                  GitHub
                </a>
              </Button>
            </div>
          </div>

          {/* Right column — live query terminal */}
          <div className="hidden lg:block">
            <QueryTerminal />
          </div>
        </div>
      </div>

      <CapabilityTicker />
    </section>
  );
}
