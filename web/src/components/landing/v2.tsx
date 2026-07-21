import {
  ArrowRightIcon,
  ArrowsClockwiseIcon,
  GithubLogoIcon,
  ProhibitIcon,
} from "@phosphor-icons/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "#/components/ui/button";
import { CommandLine, CornerMarks } from "./decor";
import { RustCode } from "./rust-code";
import { VersionSwitcher } from "./version-switcher";

/* ══ V2 · CONTROL ROOM ══════════════════════════════════════════════
   Don't describe the cache — hand the visitor the controls. The hero
   is a live model of what gpui-query does in an app: entries age, go
   stale, and revalidate in the background, with an event log.       */

type EntryState = "fresh" | "stale" | "revalidating";

interface Entry {
  key: string;
  ttlMs: number;
  state: EntryState;
  expiresAt: number;
  revalidateAt: number;
  doneAt: number;
  lastMs: number;
}

interface LogLine {
  id: number;
  at: number;
  text: string;
  kind: "net" | "state" | "user";
}

interface Sim {
  now: number;
  entries: Entry[];
  logs: LogLine[];
  fetches: number;
}

const INITIAL_ENTRIES: Entry[] = [
  {
    key: "users",
    ttlMs: 9000,
    state: "fresh",
    expiresAt: 5200,
    revalidateAt: 0,
    doneAt: 0,
    lastMs: 142,
  },
  {
    key: "repo:zed/zed",
    ttlMs: 14000,
    state: "fresh",
    expiresAt: 11800,
    revalidateAt: 0,
    doneAt: 0,
    lastMs: 96,
  },
  {
    key: "releases?page=2",
    ttlMs: 11000,
    state: "fresh",
    expiresAt: 4200,
    revalidateAt: 0,
    doneAt: 0,
    lastMs: 175,
  },
];

const INITIAL_SIM: Sim = {
  now: 0,
  entries: INITIAL_ENTRIES,
  logs: [
    { id: 1, at: 0, text: "observers registered · 3", kind: "state" },
    { id: 0, at: 0, text: "QueryClient::new() · gc 300s · policy swr", kind: "state" },
  ],
  fetches: 3,
};

const MAX_LOGS = 9;
let logSeq = 2;

function pushLog(logs: LogLine[], at: number, text: string, kind: LogLine["kind"]): LogLine[] {
  logSeq += 1;
  return [{ id: logSeq, at, text, kind }, ...logs].slice(0, MAX_LOGS);
}

function tick(prev: Sim, now: number): Sim {
  let logs = prev.logs;
  let fetches = prev.fetches;
  const entries = prev.entries.map((e) => {
    if (e.state === "fresh" && now >= e.expiresAt) {
      logs = pushLog(logs, now, `${e.key} · ttl elapsed → stale`, "state");
      return { ...e, state: "stale" as const, revalidateAt: now + 1400 + Math.random() * 1200 };
    }
    if (e.state === "stale" && now >= e.revalidateAt) {
      logs = pushLog(logs, now, `revalidate ${e.key} · background`, "state");
      return { ...e, state: "revalidating" as const, doneAt: now + 480 + Math.random() * 420 };
    }
    if (e.state === "revalidating" && now >= e.doneAt) {
      const lastMs = Math.floor(60 + Math.random() * 130);
      fetches += 1;
      logs = pushLog(logs, now, `GET ${e.key} → 200 · ${String(lastMs)}ms`, "net");
      return { ...e, state: "fresh" as const, lastMs, expiresAt: now + e.ttlMs };
    }
    return e;
  });
  return { now, entries, logs, fetches };
}

function useCacheSim() {
  const [sim, setSim] = useState<Sim>(INITIAL_SIM);
  const t0 = useRef(0);
  const reduced = useRef(false);

  useEffect(() => {
    t0.current = Date.now();
    reduced.current = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced.current) return;
    const id = window.setInterval(() => {
      if (document.hidden) return;
      const now = Date.now() - t0.current;
      setSim((prev) => tick(prev, now));
    }, 150);
    return () => clearInterval(id);
  }, []);

  const refetch = useCallback((key: string) => {
    const now = Date.now() - t0.current;
    setSim((prev) => {
      const logs = pushLog(prev.logs, now, `→ refetch("${key}") · manual`, "user");
      const entries = prev.entries.map((e) => {
        if (e.key !== key) return e;
        if (reduced.current) {
          return { ...e, state: "fresh" as const, expiresAt: now + e.ttlMs, lastMs: 118 };
        }
        return { ...e, state: "revalidating" as const, doneAt: now + 520 };
      });
      return { ...prev, entries, logs, fetches: reduced.current ? prev.fetches + 1 : prev.fetches };
    });
  }, []);

  const invalidate = useCallback((key: string) => {
    const now = Date.now() - t0.current;
    setSim((prev) => ({
      ...prev,
      logs: pushLog(prev.logs, now, `→ invalidate_queries(Exact("${key}"))`, "user"),
      entries: prev.entries.map((e) =>
        e.key === key
          ? { ...e, state: "stale" as const, expiresAt: now, revalidateAt: now + 1300 }
          : e,
      ),
    }));
  }, []);

  return { sim, refetch, invalidate };
}

/* ── Deck pieces ──────────────────────────────────────────────────── */

function fmtUptime(ms: number) {
  return `T+${(ms / 1000).toFixed(1).padStart(5, "0")}`;
}

function StateChip({ state }: { state: EntryState }) {
  if (state === "fresh") {
    return (
      <span className="inline-block border border-primary/40 bg-primary/5 px-1.5 py-0.5 font-mono text-[10px] tracking-[0.15em] text-primary">
        FRESH
      </span>
    );
  }
  if (state === "stale") {
    return (
      <span className="inline-block border border-dashed border-border px-1.5 py-0.5 font-mono text-[10px] tracking-[0.15em] text-muted-foreground">
        STALE
      </span>
    );
  }
  return (
    <span className="inline-block animate-pulse border border-primary/40 px-1.5 py-0.5 font-mono text-[10px] tracking-[0.15em] text-primary">
      REVALIDATE
    </span>
  );
}

function EntryRow({
  entry,
  now,
  onRefetch,
  onInvalidate,
}: {
  entry: Entry;
  now: number;
  onRefetch: (key: string) => void;
  onInvalidate: (key: string) => void;
}) {
  const remaining = Math.max(0, entry.expiresAt - now);
  const frac = Math.max(0, Math.min(1, remaining / entry.ttlMs));

  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border px-4 py-3.5 transition-colors last:border-b-0 hover:bg-muted/30">
      <span className="w-40 truncate font-mono text-[13px] text-foreground">{entry.key}</span>
      <span className="w-[92px]">
        <StateChip state={entry.state} />
      </span>

      <span className="flex min-w-[150px] flex-1 items-center gap-3">
        <span className="h-[3px] flex-1 bg-border" aria-hidden="true">
          {entry.state === "revalidating" ? (
            <span className="v2-meter-busy block h-full w-full opacity-60" />
          ) : (
            <span
              className="block h-full bg-primary transition-[width] duration-150 ease-linear"
              style={{ width: `${String(frac * 100)}%` }}
            />
          )}
        </span>
        <span className="w-24 text-right font-mono text-[11px] text-muted-foreground">
          {entry.state === "fresh"
            ? `ttl ${(remaining / 1000).toFixed(1)}s`
            : entry.state === "stale"
              ? "serving stale"
              : `refetching…`}
        </span>
      </span>

      <span className="flex gap-1.5">
        <button
          type="button"
          onClick={() => onRefetch(entry.key)}
          aria-label={`Refetch ${entry.key}`}
          title="refetch()"
          className="flex h-7 w-7 items-center justify-center border border-border text-muted-foreground transition-colors hover:border-primary/50 hover:text-primary focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none"
        >
          <ArrowsClockwiseIcon size={13} />
        </button>
        <button
          type="button"
          onClick={() => onInvalidate(entry.key)}
          aria-label={`Invalidate ${entry.key}`}
          title="invalidate()"
          className="flex h-7 w-7 items-center justify-center border border-border text-muted-foreground transition-colors hover:border-primary/50 hover:text-primary focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none"
        >
          <ProhibitIcon size={13} />
        </button>
      </span>
    </div>
  );
}

export function CacheDeck() {
  const { sim, refetch, invalidate } = useCacheSim();
  const inFlight = sim.entries.filter((e) => e.state === "revalidating").length;

  return (
    <div className="relative">
      <CornerMarks />
      <div className="border border-border bg-card shadow-sm">
        <div className="flex items-center gap-2 border-b border-border px-4 py-2.5">
          <span className="font-mono text-xs text-muted-foreground">
            cache://QueryClient · live
          </span>
          <span className="ml-auto flex items-center gap-2 font-mono text-[11px] tracking-widest text-primary">
            <span className="h-1.5 w-1.5 animate-pulse bg-primary" />
            {fmtUptime(sim.now)}
          </span>
        </div>

        <div className="grid lg:grid-cols-[minmax(0,7fr)_minmax(0,4fr)]">
          {/* Entries */}
          <div className="border-b border-border lg:border-r lg:border-b-0">
            {sim.entries.map((entry) => (
              <EntryRow
                key={entry.key}
                entry={entry}
                now={sim.now}
                onRefetch={refetch}
                onInvalidate={invalidate}
              />
            ))}
            <div className="flex flex-wrap gap-x-5 gap-y-1 px-4 py-3 font-mono text-[11px] text-muted-foreground">
              <span>
                entries <span className="text-foreground">{sim.entries.length}</span>
              </span>
              <span>
                in-flight <span className="text-foreground">{inFlight}</span>
              </span>
              <span>
                fetches <span className="text-foreground">{sim.fetches}</span>
              </span>
              <span>
                policy <span className="text-primary">StaleWhileRevalidate</span>
              </span>
            </div>
          </div>

          {/* Event log */}
          <div className="flex min-h-[180px] flex-col">
            <p className="border-b border-border px-4 py-2 font-mono text-[10px] tracking-[0.2em] uppercase text-muted-foreground">
              event log
            </p>
            <ul className="flex-1 space-y-1.5 overflow-hidden px-4 py-3">
              {sim.logs.map((line) => (
                <li
                  key={line.id}
                  className={`animate-status-in truncate font-mono text-[11px] ${
                    line.kind === "user"
                      ? "text-primary"
                      : line.kind === "net"
                        ? "text-foreground/85"
                        : "text-muted-foreground"
                  }`}
                >
                  <span className="text-muted-foreground/60">{fmtUptime(line.at)} </span>
                  {line.text}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ── Man page ─────────────────────────────────────────────────────── */

const MAN_ENTRIES: [string, string][] = [
  ["caching", "Ttl (default 60 s), StaleWhileRevalidate, or NoCache — declared per query."],
  ["retry", "RetryPolicy with base delay, exponential backoff, and a max-delay cap."],
  ["deduplication", "Concurrent calls with the same key share one in-flight request."],
  ["cancellation", "Fetchers receive a QuerySignal; dropped views stop their requests."],
  ["mutations", "Begin / complete / retry lifecycle with callbacks and optimistic rollback."],
  ["infinite queries", "Cursor pagination in both directions, with per-query page caps."],
  ["persistence", "Implement QueryPersister; dehydrate() and hydrate() across launches."],
  ["garbage collection", "Idle entries are collected after gc_time_ms (default 5 min)."],
  ["observers", "QueryObserver calls cx.notify() only when status actually changes."],
  ["diagnostics", "ClientDiagnostic and friends expose the client's internal state."],
];

function ManPage() {
  return (
    <section className="border-t border-border py-20 sm:py-24">
      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8">
        <div className="border border-border bg-card shadow-sm">
          <div className="flex items-center justify-between gap-4 border-b border-border px-5 py-2.5 font-mono text-[11px] text-muted-foreground">
            <span>GPUI-QUERY(3)</span>
            <span className="hidden sm:inline">Library Functions Manual</span>
            <span>GPUI-QUERY(3)</span>
          </div>

          <div className="space-y-8 p-6 sm:p-8">
            <div>
              <h2 className="font-mono text-xs tracking-[0.25em] text-foreground">NAME</h2>
              <p className="mt-2 pl-5 font-mono text-[13px] leading-6 text-muted-foreground">
                gpui-query — fetch, cache, and synchronize async data in GPUI applications
              </p>
            </div>

            <div>
              <h2 className="font-mono text-xs tracking-[0.25em] text-foreground">SYNOPSIS</h2>
              <div className="mt-2 space-y-1 overflow-x-auto pl-5 font-mono text-[13px] leading-6">
                <p>
                  <span className="text-primary">use_query</span>
                  <span className="text-foreground/85">(key, fetcher, cx)</span>
                </p>
                <p>
                  <span className="text-primary">use_mutation</span>
                  <span className="text-foreground/85">(default, cx)</span>
                </p>
                <p>
                  <span className="text-primary">use_infinite_query</span>
                  <span className="text-foreground/85">(options, fetcher, cx)</span>
                </p>
              </div>
            </div>

            <div>
              <h2 className="font-mono text-xs tracking-[0.25em] text-foreground">DESCRIPTION</h2>
              <dl className="mt-4 grid gap-px border border-border bg-border sm:grid-cols-2">
                {MAN_ENTRIES.map(([term, def]) => (
                  <div
                    key={term}
                    className="group bg-card p-4 transition-colors duration-150 hover:bg-foreground"
                  >
                    <dt className="font-mono text-[13px] text-primary group-hover:text-background">
                      {term}
                    </dt>
                    <dd className="mt-1 text-sm leading-relaxed text-muted-foreground group-hover:text-background/75">
                      {def}
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ── Before / after ───────────────────────────────────────────────── */

const HAND_ROLLED = `struct UserList {
    users: Option<Vec<User>>,
    error: Option<String>,
    loading: bool,
    generation: u64,
}

impl UserList {
    fn fetch(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.generation += 1;
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let result = fetch_users().await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return; // superseded by a newer request
                }
                this.loading = false;
                match result {
                    Ok(users) => this.users = Some(users),
                    Err(err) => this.error = Some(err.to_string()),
                }
                cx.notify();
            })
        })
        .detach();
    }
}

// still missing: caching, ttl, retry, dedup across views…`;

const WITH_QUERY = `let (users, _sub) = use_query(
    "users",
    |signal| async move { fetch_users(&signal).await },
    cx,
);

// cached · retried · deduped · revalidated · cancellable`;

export function BeforeAfter() {
  const [mode, setMode] = useState<"hand" | "query">("hand");
  const code = mode === "hand" ? HAND_ROLLED : WITH_QUERY;
  const handLines = HAND_ROLLED.split("\n").length;
  const queryLines = WITH_QUERY.split("\n").length;

  return (
    <section className="border-t border-border bg-muted/30 py-20 sm:py-24">
      <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8">
        <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
          <span className="text-primary/50">{"// "}</span>diff
        </p>
        <h2 className="font-display mt-3 text-3xl font-bold tracking-tight sm:text-4xl">
          The code you stop writing
        </h2>
        <p className="mt-4 max-w-2xl text-muted-foreground">
          The same feature — a fetched, cached user list — written both ways. Toggle to compare.
        </p>

        <div className="relative mt-10">
          <CornerMarks />
          <div className="border border-border bg-card shadow-sm">
            <div
              role="tablist"
              aria-label="Code comparison"
              className="flex border-b border-border"
            >
              <button
                type="button"
                role="tab"
                aria-selected={mode === "hand"}
                onClick={() => setMode("hand")}
                className={`border-r border-border px-4 py-3 font-mono text-xs transition-colors focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none ${
                  mode === "hand"
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                by hand · cx.spawn
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={mode === "query"}
                onClick={() => setMode("query")}
                className={`border-r border-border px-4 py-3 font-mono text-xs transition-colors focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none ${
                  mode === "query"
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                with gpui-query
              </button>
              <span className="ml-auto self-center px-4 font-mono text-[11px] text-muted-foreground">
                {mode === "hand" ? `${String(handLines)} lines` : `${String(queryLines)} lines`}
              </span>
            </div>
            <RustCode code={code} lineNumbers className="min-h-[300px] sm:min-h-[680px]" />
          </div>
        </div>

        <p className="mt-5 font-mono text-xs text-muted-foreground">
          <span className="text-primary/60">▸ </span>
          {String(handLines)} lines of lifecycle plumbing → {String(queryLines)} lines, with more
          behavior.
        </p>
      </div>
    </section>
  );
}

/* ── Hero + page ──────────────────────────────────────────────────── */

function HeroV2() {
  return (
    <section className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="bg-dot-grid pointer-events-none absolute inset-0 -z-10 [mask-image:radial-gradient(ellipse_70%_55%_at_50%_0%,black,transparent)]"
      />
      <div className="mx-auto max-w-7xl px-4 pt-16 pb-16 sm:px-6 sm:pt-24 sm:pb-20 lg:px-8">
        <div className="flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-2xl">
            <div className="inline-flex items-center gap-2.5 border border-primary/25 bg-primary/5 px-3 py-1.5 font-mono text-[11px] tracking-[0.2em] uppercase text-primary">
              <span className="h-1.5 w-1.5 animate-pulse bg-primary" />
              live demo below — no mock video
            </div>
            <h1 className="font-display mt-8 text-5xl font-bold tracking-tight text-foreground sm:text-6xl">
              Watch the cache <span className="text-primary">work</span>.
            </h1>
            <p className="mt-6 text-lg leading-relaxed text-muted-foreground">
              This deck is a running model of gpui-query inside your app: entries age, go stale, and
              revalidate in the background — without a line of lifecycle code from you.
            </p>
          </div>
          <div className="flex shrink-0 flex-col gap-4 sm:flex-row">
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

        <div className="mt-12">
          <CacheDeck />
        </div>
      </div>
    </section>
  );
}

function CtaV2() {
  return (
    <section className="border-t border-border py-24 sm:py-28">
      <div className="mx-auto max-w-2xl px-4 text-center sm:px-6">
        <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
          <span className="text-primary/50">{"// "}</span>hands off
        </p>
        <h2 className="font-display mt-3 text-4xl font-bold tracking-tight sm:text-5xl">
          Put your cache on autopilot
        </h2>
        <p className="mx-auto mt-4 max-w-md text-muted-foreground">
          Everything the deck above just did — in your GPUI app, from one dependency.
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

export function LandingV2() {
  return (
    <div className="flex flex-col">
      <HeroV2 />
      <ManPage />
      <BeforeAfter />
      <CtaV2 />
      <VersionSwitcher current="v2" />
    </div>
  );
}
