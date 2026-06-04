import { ArrowRightIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";

const heroCode = `use gpui_query::prelude::*;

fn render_users(cx: &mut ViewContext<App>) {
  let query = use_query(cx, "user-list", || async {
    fetch_users().await
  });

  div().children(match &query.data {
    Some(users) => users.iter().map(render_user),
    None => vec![div().child("Loading...")],
  })
}`;

export function HeroSection() {
  return (
    <section className="relative overflow-hidden">
      {/* Subtle radial glow — top-left only */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -left-40 -top-40 -z-10 h-[600px] w-[600px] bg-primary/5 blur-[120px]"
      />

      <div className="mx-auto max-w-7xl px-4 pb-20 pt-24 sm:px-6 sm:pb-28 sm:pt-32 lg:px-8">
        <div className="flex items-start gap-16">
          {/* Left column */}
          <div className="max-w-xl flex-shrink-0">
            {/* Badge — static, no ping */}
            <div className="mb-8 inline-flex items-center gap-2 border border-primary/20 bg-primary/5 px-3 py-1 text-xs font-medium tracking-wider uppercase text-primary">
              <span className="inline-flex h-1.5 w-1.5 bg-primary" />
              Open-source async state for Rust GPUI
            </div>

            <h1
              className="font-[family-name:var(--font-display)] text-4xl font-bold tracking-tight text-foreground sm:text-5xl lg:text-6xl"
            >
              Async state management that gets out of your way
            </h1>

            <p className="mt-6 max-w-lg text-lg text-muted-foreground">
              Fetch, cache, and synchronize data in GPUI with a single hook.
              Purpose-built for the Zed editor framework.
            </p>

            {/* Nested CTAs */}
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

          {/* Right column — code preview */}
          <div className="hidden flex-1 lg:block">
            <div className="ring-1 ring-foreground/8 bg-card shadow-sm">
              {/* Terminal header */}
              <div className="flex items-center gap-2 border-b border-border px-4 py-3">
                <div className="flex gap-1.5">
                  <span className="h-2.5 w-2.5 bg-foreground/10" />
                  <span className="h-2.5 w-2.5 bg-foreground/10" />
                  <span className="h-2.5 w-2.5 bg-foreground/10" />
                </div>
                <span className="ml-2 text-xs text-muted-foreground font-mono">
                  src/main.rs
                </span>
              </div>
              {/* Code content */}
              <div className="bg-muted/50 p-6 font-mono text-sm leading-relaxed text-foreground/80">
                <pre className="whitespace-pre">{heroCode}</pre>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
