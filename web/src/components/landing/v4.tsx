import { ArrowRightIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { useCallback, useRef } from "react";
import { Button } from "#/components/ui/button";
import { ArchitectureSection } from "./architecture-section";
import { ComparisonSection } from "./comparison-section";
import { CommandLine } from "./decor";
import { FeatureShowcase } from "./feature-showcase";
import { HookTabs, SchematicPanel } from "./v1";
import { BeforeAfter, CacheDeck } from "./v2";
import { AnnotatedFigure } from "./v3";
import { VersionSwitcher } from "./version-switcher";

function HeroV4() {
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
              async state for GPUI
            </div>

            <h1 className="font-display mt-8 text-5xl font-bold tracking-tight text-foreground sm:text-6xl">
              Cache, fetch,
              <br />
              render. <span className="text-primary">One path.</span>
            </h1>

            <p className="mt-6 max-w-lg text-lg leading-relaxed text-muted-foreground">
              gpui-query wires network, cache, retry, and revalidation into one small API for GPUI
              applications.
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

          <SchematicPanel />
        </div>
      </div>
    </section>
  );
}

function QueryClientLiveSection() {
  return (
    <section className="border-t border-border py-20 sm:py-24">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <h2 className="font-display text-3xl font-bold tracking-tight sm:text-4xl">
          QueryClient, live
        </h2>
        <p className="mt-4 max-w-2xl text-muted-foreground">
          Entries age, go stale, revalidate, and log each transition. Use the row controls to force
          the lifecycle.
        </p>
        <div className="mt-10">
          <CacheDeck />
        </div>
      </div>
    </section>
  );
}

function CtaV4() {
  return (
    <section className="relative overflow-hidden border-t border-border py-24 sm:py-28">
      <div
        aria-hidden="true"
        className="v1-grid-base pointer-events-none absolute inset-0 -z-10 [mask-image:radial-gradient(ellipse_70%_70%_at_50%_100%,black,transparent)]"
      />
      <div className="mx-auto max-w-2xl px-4 text-center sm:px-6">
        <h2 className="font-display text-4xl font-bold tracking-tight sm:text-5xl">
          Ship the lifecycle once
        </h2>
        <p className="mx-auto mt-4 max-w-md text-muted-foreground">
          Add the crate, keep your views small, and let QueryClient own the async state machine.
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

export function LandingV4() {
  return (
    <div className="flex flex-col">
      <HeroV4 />
      <FeatureShowcase />
      <QueryClientLiveSection />
      <HookTabs />
      <BeforeAfter />
      <ComparisonSection />
      <AnnotatedFigure />
      <ArchitectureSection />
      <CtaV4 />
      <VersionSwitcher current="v4" />
    </div>
  );
}
