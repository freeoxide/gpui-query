import { createFileRoute } from "@tanstack/react-router";
import {
  Accordion,
  AccordionItem,
  AccordionTrigger,
  AccordionContent,
} from "#/components/ui/accordion";
import { faqPage } from "#/lib/seo";

const faqItems = [
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
    question: "How do I set up QueryClient in my app?",
    answer:
      "Create a QueryClient instance and register it in your GPUI application. The client manages all query resources, caching, and garbage collection. See the Getting Started guide for a complete walkthrough.",
  },
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
    question: "What is QuerySignal and when do I check it?",
    answer:
      "QuerySignal is an Arc<AtomicBool> that enables cooperative cancellation. Long-running queries should check the signal periodically (especially between retry attempts) and abort early if cancelled.",
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
];

export const Route = createFileRoute("/faq")({
  head: () => ({
    meta: [
      { title: "FAQ - gpui-query" },
      {
        name: "description",
        content: "Frequently asked questions about gpui-query — async state management for GPUI.",
      },
    ],
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

function FAQPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-3xl">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
          Frequently Asked Questions
        </h1>
        <p className="mt-4 text-lg text-muted-foreground">
          Everything you need to know about gpui-query.
        </p>

        <div className="mt-10">
          <Accordion type="multiple">
            {faqItems.map((item, index) => (
              <AccordionItem key={index} value={`faq-${index}`}>
                <AccordionTrigger>{item.question}</AccordionTrigger>
                <AccordionContent>
                  <p className="text-muted-foreground leading-relaxed">{item.answer}</p>
                </AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </div>
      </div>
    </div>
  );
}
