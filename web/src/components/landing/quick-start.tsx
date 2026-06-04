import { ArrowRightIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";
import { CodeBlock } from "#/components/code-block";

const steps = [
  {
    num: 1,
    label: "Add the dependency",
    detail: "gpui-query to your Cargo.toml",
  },
  {
    num: 2,
    label: "Write your first query",
    detail: "Call use_query with a key and async fetcher",
  },
  {
    num: 3,
    label: "Done",
    detail: "Caching, retry, and deduplication are automatic",
  },
];

export function QuickStart() {
  return (
    <section className="border-t border-border py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="grid items-start gap-12 lg:grid-cols-2 lg:gap-16">
          {/* Left — steps */}
          <div data-reveal>
            <p className="text-xs font-medium tracking-widest uppercase text-primary">
              Get Started
            </p>
            <h2 className="mt-2 text-3xl font-bold tracking-tight sm:text-4xl">
              Up and running in 30 seconds
            </h2>

            <ol className="mt-10 space-y-6">
              {steps.map((step) => (
                <li key={step.num} className="flex items-start gap-4">
                  <span className="flex h-8 w-8 shrink-0 items-center justify-center border border-primary/30 text-sm font-mono text-primary">
                    {step.num}
                  </span>
                  <div>
                    <p className="text-sm font-semibold text-foreground">
                      {step.label}
                    </p>
                    <p className="mt-0.5 text-sm text-muted-foreground">
                      {step.detail}
                    </p>
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

          {/* Right — code blocks */}
          <div data-reveal data-reveal="slide-right">
            <CodeBlock title="Cargo.toml">
              <pre className="overflow-x-auto border border-t-0 bg-muted/50 p-4 text-sm leading-relaxed font-mono">
                <code>{`[dependencies]
gpui-query = "0.1"`}</code>
              </pre>
            </CodeBlock>

            <div className="mt-6">
              <CodeBlock title="src/main.rs">
                <pre className="overflow-x-auto border border-t-0 bg-muted/50 p-4 text-sm leading-relaxed font-mono">
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
        </div>
      </div>
    </section>
  );
}
