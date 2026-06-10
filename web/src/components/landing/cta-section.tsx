import { ArrowRightIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";
import { CommandLine } from "./decor";

export function CtaSection() {
  return (
    <section className="relative overflow-hidden border-t border-border py-24 sm:py-32">
      {/* Dot grid rising from the footer */}
      <div
        aria-hidden="true"
        className="bg-dot-grid pointer-events-none absolute inset-0 -z-10 [mask-image:radial-gradient(ellipse_70%_70%_at_50%_100%,black,transparent)]"
      />

      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-2xl text-center" data-reveal>
          <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
            <span className="text-primary/50">{"// "}</span>
            ship it
          </p>
          <h2 className="font-display mt-3 text-4xl font-bold tracking-tight sm:text-5xl">
            Stop hand-rolling async state
          </h2>
          <p className="mx-auto mt-4 max-w-md text-muted-foreground">
            One dependency away from caching, retries, and revalidation in your GPUI app.
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
                href="https://github.com/hmziqrs/gpui-query"
                target="_blank"
                rel="noopener noreferrer"
              >
                <GithubLogoIcon size={18} className="mr-1.5" />
                Star on GitHub
              </a>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}
