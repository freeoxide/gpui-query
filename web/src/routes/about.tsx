import { createFileRoute, Link } from "@tanstack/react-router";
import {
  Github,
  ExternalLink,
  Layers,
  BookOpen,
  Cpu,
  Database,
  Code2,
  Package,
  Shield,
  FileText,
  PenLine,
  User,
  Heart,
} from "lucide-react";
import { Button } from "#/components/ui/button";
import { Card, CardHeader, CardContent } from "#/components/ui/card";
import { Badge } from "#/components/ui/badge";
import { Separator } from "#/components/ui/separator";

export const Route = createFileRoute("/about")({
  head: () => ({
    meta: [
      { title: "About - gpui-query" },
      {
        name: "description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
      { property: "og:title", content: "About - gpui-query" },
      {
        property: "og:description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary" },
      { name: "twitter:title", content: "About - gpui-query" },
      {
        name: "twitter:description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.hmziq.xyz/about" }],
  }),
  component: AboutPage,
});

function AboutPage() {
  return (
    <div className="flex flex-col">
      <HeroSection />
      <WhatIsSection />
      <AuthorSection />
      <TechStackSection />
      <OpenSourceSection />
      <LinksSection />
    </div>
  );
}

/* ─── Hero ─────────────────────────────────────────────────────── */

function HeroSection() {
  return (
    <section className="bg-gradient-to-b from-primary/5 to-transparent py-16 text-center">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-primary/20 bg-primary/5 px-4 py-1.5 text-sm font-medium text-primary">
            <Heart className="h-3.5 w-3.5" />
            Built with care for the Rust ecosystem
          </div>
          <h1 className="text-4xl font-extrabold tracking-tight sm:text-5xl">
            About{" "}
            <span
              className="bg-gradient-to-r from-primary via-primary/80 to-emerald-400 bg-clip-text text-transparent"
              style={{
                backgroundSize: "200% auto",
                animation: "gradient-shift 6s ease infinite",
              }}
            >
              gpui-query
            </span>
          </h1>
          <p className="mx-auto mt-4 max-w-xl text-lg text-muted-foreground">
            The story behind async state management for GPUI
          </p>
        </div>
      </div>
    </section>
  );
}

/* ─── What Is ──────────────────────────────────────────────────── */

function WhatIsSection() {
  return (
    <section className="border-t border-primary/5 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <Card className="border-primary/10">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <Layers className="h-5 w-5" />
                </div>
                <h2 className="text-2xl font-bold tracking-tight">What is gpui-query?</h2>
              </div>
            </CardHeader>
            <CardContent className="space-y-4 leading-relaxed text-muted-foreground">
              <p>
                <strong className="text-foreground">gpui-query</strong> brings the reactive query
                patterns you love from the web world into Rust GPUI applications. Fetch, cache, and
                synchronize async data with a single hook — no manual lifecycle management required.
              </p>
              <p>
                Built on a layered architecture with a framework-agnostic Core, a Client layer for
                caching and garbage collection, and a Hook layer that integrates directly with
                GPUI's reactive primitives, gpui-query is designed to feel natural in the Rust
                ecosystem while providing battle-tested patterns from TanStack Query.
              </p>
              <p>
                Whether you are building plugins for the{" "}
                <a
                  href="https://zed.dev"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-medium text-primary underline-offset-4 hover:underline"
                >
                  Zed editor
                </a>{" "}
                or crafting your own GPUI application, gpui-query eliminates the boilerplate of
                managing asynchronous data so you can focus on shipping features.
              </p>
            </CardContent>
          </Card>
        </div>
      </div>
    </section>
  );
}

/* ─── Author ───────────────────────────────────────────────────── */

function AuthorSection() {
  return (
    <section className="border-t border-primary/5 bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <Card className="border-primary/10">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <User className="h-5 w-5" />
                </div>
                <h2 className="text-2xl font-bold tracking-tight">The Author</h2>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
                {/* Avatar placeholder */}
                <div className="flex h-20 w-20 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-primary/20 to-emerald-500/20 text-2xl font-bold text-primary ring-2 ring-primary/10">
                  H
                </div>
                <div className="space-y-3 text-center sm:text-left">
                  <div>
                    <h3 className="text-lg font-semibold">hmziq</h3>
                    <p className="text-sm text-muted-foreground">
                      Rust developer &amp; open-source contributor
                    </p>
                  </div>
                  <p className="text-sm leading-relaxed text-muted-foreground">
                    Created and maintains gpui-query. Passionate about building ergonomic developer
                    tools that make complex async patterns accessible to everyone in the Rust
                    ecosystem.
                  </p>
                  <Button variant="outline" size="sm" asChild>
                    <a href="https://github.com/hmziqrs" target="_blank" rel="noopener noreferrer">
                      <Github className="mr-1.5 h-4 w-4" />
                      @hmziqrs
                      <ExternalLink className="ml-1 h-3 w-3" />
                    </a>
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </section>
  );
}

/* ─── Tech Stack ───────────────────────────────────────────────── */

const techStack = [
  {
    icon: Cpu,
    label: "GPUI",
    description: "Zed's GPU-accelerated UI framework",
  },
  {
    icon: Code2,
    label: "Rust",
    description: "Memory-safe, blazing fast",
  },
  {
    icon: Database,
    label: "Query Cache",
    description: "TTL, SWR, LatestWins policies",
  },
  {
    icon: Layers,
    label: "3-Layer Arch",
    description: "Core, Client, Hook separation",
  },
  {
    icon: Shield,
    label: "Type Safe",
    description: "Compile-time guarantees",
  },
  {
    icon: Package,
    label: "Crate.io",
    description: "Published Rust package",
  },
];

function TechStackSection() {
  return (
    <section className="border-t border-primary/5 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">Technology Stack</h2>
          <p className="mx-auto mt-4 max-w-2xl text-muted-foreground">
            Built with proven technologies and patterns for reliable async state management.
          </p>
        </div>

        <div className="mx-auto mt-14 grid max-w-3xl grid-cols-2 gap-4 sm:grid-cols-3">
          {techStack.map((tech) => (
            <div
              key={tech.label}
              className="group flex flex-col items-center rounded-xl border border-primary/10 bg-card p-5 text-center transition-all duration-200 hover:-translate-y-1 hover:shadow-lg hover:shadow-primary/5"
            >
              <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10 text-primary transition-colors group-hover:bg-primary/15">
                <tech.icon className="h-6 w-6" />
              </div>
              <h3 className="text-sm font-semibold">{tech.label}</h3>
              <p className="mt-1 text-xs text-muted-foreground">{tech.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ─── Open Source ──────────────────────────────────────────────── */

function OpenSourceSection() {
  return (
    <section className="border-t border-primary/5 bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <Card className="border-primary/10">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
                  <Heart className="h-5 w-5" />
                </div>
                <h2 className="text-2xl font-bold tracking-tight">Open Source</h2>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex flex-wrap items-center gap-3">
                <Badge className="bg-emerald-600 text-white hover:bg-emerald-700">
                  MIT License
                </Badge>
                <span className="text-sm text-muted-foreground">
                  Free to use, modify, and distribute
                </span>
              </div>
              <Separator />
              <p className="leading-relaxed text-muted-foreground">
                gpui-query is open-source software released under the{" "}
                <strong className="text-foreground">MIT License</strong>. You are free to use it in
                personal projects, commercial applications, and anything in between. Contributions,
                bug reports, and feature requests are always welcome on GitHub.
              </p>
              <div className="flex flex-wrap gap-3 pt-2">
                <Button size="sm" asChild>
                  <a
                    href="https://github.com/hmziqrs/gpui-query"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <Github className="mr-1.5 h-4 w-4" />
                    Star on GitHub
                  </a>
                </Button>
                <Button variant="outline" size="sm" asChild>
                  <a
                    href="https://github.com/hmziqrs/gpui-query/issues"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Report an Issue
                    <ExternalLink className="ml-1.5 h-3 w-3" />
                  </a>
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </section>
  );
}

/* ─── Links ────────────────────────────────────────────────────── */

function LinkCard({
  icon: Icon,
  label,
  description,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  description: string;
}) {
  return (
    <div className="group flex items-start gap-4 rounded-xl border border-primary/10 bg-card p-5 transition-all duration-200 hover:-translate-y-1 hover:shadow-lg hover:shadow-primary/5">
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary transition-colors group-hover:bg-primary/15">
        <Icon className="h-5 w-5" />
      </div>
      <div className="min-w-0">
        <h3 className="flex items-center gap-1.5 font-semibold">
          {label}
          <ExternalLink className="h-3 w-3 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
        </h3>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

function LinksSection() {
  return (
    <section className="border-t border-primary/5 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">Explore More</h2>
          <p className="mx-auto mt-4 max-w-2xl text-muted-foreground">
            Dive deeper into gpui-query with these resources.
          </p>
        </div>

        <div className="mx-auto mt-14 grid max-w-3xl gap-4 sm:grid-cols-2">
          <a href="https://github.com/hmziqrs/gpui-query" target="_blank" rel="noopener noreferrer">
            <LinkCard
              icon={Github}
              label="GitHub"
              description="Source code, issues, and releases"
            />
          </a>

          <a href="/docs/">
            <LinkCard
              icon={BookOpen}
              label="Documentation"
              description="Guides, API reference, and examples"
            />
          </a>

          <a href="https://gpui.rs/blog">
            <LinkCard icon={PenLine} label="Blog" description="Announcements and deep dives" />
          </a>

          <Link to="/changelog">
            <LinkCard
              icon={FileText}
              label="Changelog"
              description="Release history and breaking changes"
            />
          </Link>
        </div>
      </div>
    </section>
  );
}
