import { createFileRoute } from "@tanstack/react-router";
import {
  Accordion,
  AccordionItem,
  AccordionTrigger,
  AccordionContent,
} from "#/components/ui/accordion";
import { Button } from "#/components/ui/button";
import { faqPage } from "#/lib/seo";
import { ArrowRight, Github, Rocket, Building2, Wrench } from "lucide-react";

/* ─── FAQ Data ──────────────────────────────────────────────────────── */

interface FaqItem {
  question: string;
  answer: string;
}

interface FaqCategory {
  label: string;
  icon: React.ElementType;
  items: FaqItem[];
}

const categories: FaqCategory[] = [
  {
    label: "Getting Started",
    icon: Rocket,
    items: [
      {
        question: "How is gpui-query different from TanStack Query?",
        answer:
          "gpui-query adapts TanStack Query's proven patterns to Rust and the GPUI framework. It uses Rust's type system for compile-time guarantees, Arc<AtomicBool> for cooperative cancellation, and integrates directly with GPUI's render loop.",
      },
      {
        question: "Can I use gpui-query outside of Zed?",
        answer:
          "gpui-query is designed for the GPUI framework, which powers the Zed editor. While architecturally the Core layer is framework-agnostic, the Hook layer depends on GPUI's reactive primitives.",
      },
      {
        question: "How do I set up QueryClient in my app?",
        answer:
          "Create a QueryClient instance and register it in your GPUI application. The client manages all query resources, caching, and garbage collection. See the Getting Started guide for a complete walkthrough.",
      },
    ],
  },
  {
    label: "Architecture",
    icon: Building2,
    items: [
      {
        question: "Why does use_query return a tuple instead of an object?",
        answer:
          "Following Rust convention, use_query returns a tuple (data, status) for ergonomic destructuring. The status enum provides complete lifecycle information.",
      },
      {
        question: "What happens if my component unmounts during a fetch?",
        answer:
          "gpui-query uses cooperative cancellation via QuerySignal (Arc<AtomicBool>). When a component unmounts, the signal is set and the query checks it between retry attempts, enabling clean teardown.",
      },
      {
        question: "What is QuerySignal and when do I check it?",
        answer:
          "QuerySignal is an Arc<AtomicBool> that enables cooperative cancellation. Long-running queries should check the signal periodically (especially between retry attempts) and abort early if cancelled.",
      },
    ],
  },
  {
    label: "Advanced",
    icon: Wrench,
    items: [
      {
        question: "Why does LatestWins cancel my in-flight request?",
        answer:
          "LatestWins is a RequestPolicy that ensures only the most recent request's result is used. When a new request arrives, previous in-flight requests are cancelled via their signals, preventing stale data from overwriting fresh results.",
      },
      {
        question: "How do I persist my query cache?",
        answer:
          "Implement the QueryPersister trait to serialize and restore query state. gpui-query supports custom persistence backends — you can use files, databases, or any storage mechanism.",
      },
      {
        question: "How do I handle pagination?",
        answer:
          "Use use_infinite_query for paginated data. It supports bidirectional fetching (fetch_next_page_infinite / fetch_previous_page_infinite) and configurable max_pages to limit cached pages.",
      },
      {
        question: "Is there a devtools experience?",
        answer:
          "gpui-query provides ClientDiagnostic types and dehydrate/hydrate methods for inspecting cache state, query status, and resource lifecycle — a developer toolkit for debugging async state.",
      },
    ],
  },
];

/* ─── Flat list for JSON-LD ─────────────────────────────────────────── */

const faqItems: FaqItem[] = categories.flatMap((c) => c.items);

/* ─── Route ─────────────────────────────────────────────────────────── */

export const Route = createFileRoute("/faq")({
  head: () => ({
    meta: [
      { title: "FAQ - gpui-query" },
      {
        name: "description",
        content: "Frequently asked questions about gpui-query — async state management for GPUI.",
      },
      { property: "og:title", content: "FAQ - gpui-query" },
      {
        property: "og:description",
        content: "Frequently asked questions about gpui-query — async state management for GPUI.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://gpui-query.hmziq.xyz/faq" },
      { property: "og:image", content: "https://gpui-query.hmziq.xyz/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "FAQ - gpui-query" },
      {
        name: "twitter:description",
        content: "Frequently asked questions about gpui-query — async state management for GPUI.",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/faq" }],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify(
          faqPage({
            questions: faqItems.map((item) => ({
              question: item.question,
              answer: item.answer,
            })),
          }),
        ),
      },
    ],
  }),
  component: FAQPage,
});

/* ─── Page Component ────────────────────────────────────────────────── */

function FAQPage() {
  let globalIndex = 0;

  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 sm:py-16 lg:px-8">
      <div className="mx-auto max-w-3xl">
        {/* Header */}
        <div className="border-l-4 border-primary pl-5">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Frequently Asked Questions
          </h1>
          <p className="mt-2 text-lg text-muted-foreground">
            Everything you need to know about gpui-query, organized by topic.
          </p>
        </div>

        {/* Category sections */}
        <div className="mt-14 space-y-14">
          {categories.map((category) => {
            const startIdx = globalIndex;
            globalIndex += category.items.length;

            return (
              <section key={category.label}>
                {/* Category heading with left border accent */}
                <div className="flex items-center gap-3 border-l-4 border-primary pl-4">
                  <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                    <category.icon className="h-4 w-4" />
                  </div>
                  <h2 className="text-xl font-semibold tracking-tight sm:text-2xl">
                    {category.label}
                  </h2>
                </div>

                {/* Accordion for this category */}
                <div className="mt-6">
                  <Accordion multiple>
                    {category.items.map((item, idx) => (
                      <AccordionItem
                        key={idx}
                        value={`faq-${startIdx + idx}`}
                        className="group border-border/60 transition-colors hover:border-primary/30"
                      >
                        <AccordionTrigger className="text-left text-[15px] font-medium text-foreground/90 transition-colors duration-200 hover:text-primary [&[data-state=open]>svg]:rotate-180 [&[data-state=open]]:text-primary">
                          {item.question}
                        </AccordionTrigger>
                        <AccordionContent className="overflow-hidden text-sm data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down">
                          <p className="leading-relaxed text-muted-foreground">{item.answer}</p>
                        </AccordionContent>
                      </AccordionItem>
                    ))}
                  </Accordion>
                </div>
              </section>
            );
          })}
        </div>

        {/* CTA */}
        <div className="mt-20 rounded-2xl bg-gradient-to-br from-primary/10 via-primary/5 to-emerald-500/10 px-8 py-14 text-center sm:px-16">
          <h2 className="text-2xl font-bold tracking-tight sm:text-3xl">Still have questions?</h2>
          <p className="mx-auto mt-3 max-w-md text-muted-foreground">
            Open an issue on GitHub and we will get back to you. Contributions and discussions are
            always welcome.
          </p>
          <div className="mt-8 flex flex-col items-center justify-center gap-4 sm:flex-row">
            <Button
              size="lg"
              asChild
              className="px-6 text-base font-semibold shadow-lg shadow-primary/20"
            >
              <a
                href="https://github.com/hmziqrs/gpui-query/issues"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="mr-2 h-4 w-4" />
                Open an Issue
                <ArrowRight className="ml-2 h-4 w-4" />
              </a>
            </Button>

            <Button variant="outline" size="lg" asChild className="px-6 text-base font-semibold">
              <a href="/docs/">
                Read the Docs
                <ArrowRight className="ml-2 h-4 w-4" />
              </a>
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
