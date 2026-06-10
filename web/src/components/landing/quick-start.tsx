import { ArrowRightIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";
import { CommandLine, CornerMarks, SectionHeading } from "./decor";

const steps = [
  {
    num: "01",
    label: "Add the dependency",
    detail: "One cargo command, zero configuration.",
  },
  {
    num: "02",
    label: "Write your first query",
    detail: "Call use_query with a key and an async fetcher.",
  },
  {
    num: "03",
    label: "Done",
    detail: "Caching, retry, and deduplication are automatic.",
  },
];

const mainCode = `use gpui_query::prelude::*;

fn render_user_list(cx: &mut ViewContext<App>) -> impl IntoElement {
    let query = use_query(cx, "user-list", || async {
        fetch_users().await
    });

    div().children(match &query.data {
        Some(users) => users.iter().map(render_user),
        None => vec![div().child("Loading...")],
    })
}`;

export function QuickStart() {
  return (
    <section className="py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="grid items-start gap-12 lg:grid-cols-2 lg:gap-16">
          {/* Left — steps */}
          <div data-reveal>
            <SectionHeading eyebrow="get started" title="Up and running in 30 seconds" />

            <ol className="mt-10 space-y-6">
              {steps.map((step) => (
                <li key={step.num} className="flex items-start gap-4">
                  <span className="flex h-9 w-9 shrink-0 items-center justify-center border border-primary/30 bg-primary/5 font-mono text-xs text-primary">
                    {step.num}
                  </span>
                  <div>
                    <p className="text-sm font-semibold text-foreground">{step.label}</p>
                    <p className="mt-0.5 text-sm text-muted-foreground">{step.detail}</p>
                  </div>
                </li>
              ))}
            </ol>

            <div className="mt-10">
              <Button variant="outline" asChild>
                <a href="/docs/">
                  Read the full guide
                  <ArrowRightIcon size={14} className="ml-1" />
                </a>
              </Button>
            </div>
          </div>

          {/* Right — install command + code */}
          <div data-reveal="slide-right" className="space-y-6">
            <CommandLine command="cargo add gpui-query" />

            <div className="relative">
              <CornerMarks />
              <div className="border border-border bg-card shadow-sm">
                <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
                  <span className="font-mono text-xs text-muted-foreground">src/main.rs</span>
                  <span className="font-mono text-[10px] tracking-[0.2em] uppercase text-primary/70">
                    rust
                  </span>
                </div>
                <pre className="overflow-x-auto p-5 font-mono text-[13px] leading-6 text-foreground/85">
                  <code>{mainCode}</code>
                </pre>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
