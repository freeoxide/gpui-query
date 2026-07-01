import { createFileRoute, Link } from "@tanstack/react-router";
import {
  GithubLogoIcon,
  ArrowUpRightIcon,
  StackIcon,
  BookOpenIcon,
  CpuIcon,
  DatabaseIcon,
  CodeIcon,
  PackageIcon,
  ShieldCheckIcon,
  FileTextIcon,
  NotePencilIcon,
  UserIcon,
  HeartIcon,
  GlobeIcon,
  XLogoIcon,
  LightningIcon,
} from "@phosphor-icons/react";
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
      { property: "og:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: "About - gpui-query" },
      {
        name: "twitter:description",
        content:
          "About gpui-query — zero-boilerplate async state management for the GPUI framework.",
      },
      { name: "twitter:image", content: "https://gpui-query.freeoxide.com/og-image.png" },
    ],
    links: [{ rel: "canonical", href: "https://gpui-query.freeoxide.com/about" }],
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
          <div className="mb-4 inline-flex items-center gap-2 border border-primary/20 bg-primary/5 px-3 py-1 text-xs font-medium tracking-wider uppercase text-primary">
            <HeartIcon size={14} />
            Built with care for the Rust ecosystem
          </div>
          <h1 className="text-4xl font-extrabold tracking-tight sm:text-5xl">
            About <span className="text-primary">gpui-query</span>
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
    <section className="border-t border-border py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <Card className="border-primary/10">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center bg-primary/10 text-primary">
                  <StackIcon size={20} />
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
    <section className="border-t border-border bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <Card className="border-primary/10">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center bg-primary/10 text-primary">
                  <UserIcon size={20} />
                </div>
                <h2 className="text-2xl font-bold tracking-tight">The Author</h2>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
                {/* Avatar */}
                <div className="flex h-20 w-20 shrink-0 items-center justify-center bg-gradient-to-br from-primary/20 to-emerald-500/20 text-2xl font-bold text-primary ring-2 ring-primary/10">
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
                  {/* Author links */}
                  <div className="flex flex-wrap gap-2">
                    <Button variant="outline" size="sm" asChild>
                      <a
                        href="https://github.com/hmziqrs"
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        <GithubLogoIcon size={16} className="mr-1.5" />
                        @hmziqrs
                      </a>
                    </Button>
                    <Button variant="outline" size="sm" asChild>
                      <a href="https://x.com/hmziqrs" target="_blank" rel="noopener noreferrer">
                        <XLogoIcon size={16} className="mr-1.5" />
                        @hmziqrs
                      </a>
                    </Button>
                    <Button variant="outline" size="sm" asChild>
                      <a href="https://hmziq.rs" target="_blank" rel="noopener noreferrer">
                        <GlobeIcon size={16} className="mr-1.5" />
                        hmziq.rs
                      </a>
                    </Button>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Related projects */}
          <div className="mt-8 grid gap-4 sm:grid-cols-2">
            <a
              href="https://freeoxide.com"
              target="_blank"
              rel="noopener noreferrer"
              className="group flex items-start gap-4 bg-card p-5 ring-1 ring-foreground/8 transition-colors duration-200 hover:ring-primary/25"
            >
              <div className="flex h-10 w-10 shrink-0 items-center justify-center bg-primary/10 text-primary">
                <LightningIcon size={20} weight="duotone" />
              </div>
              <div>
                <h3 className="flex items-center gap-1.5 font-semibold text-sm">
                  freeoxide.com
                  <ArrowUpRightIcon
                    size={12}
                    className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                  />
                </h3>
                <p className="mt-1 text-xs text-muted-foreground">
                  A project born from the gpui-query initiative
                </p>
              </div>
            </a>

            <a
              href="https://hmziq.xyz"
              target="_blank"
              rel="noopener noreferrer"
              className="group flex items-start gap-4 bg-card p-5 ring-1 ring-foreground/8 transition-colors duration-200 hover:ring-primary/25"
            >
              <div className="flex h-10 w-10 shrink-0 items-center justify-center bg-primary/10 text-primary">
                <GlobeIcon size={20} weight="duotone" />
              </div>
              <div>
                <h3 className="flex items-center gap-1.5 font-semibold text-sm">
                  hmziq.xyz
                  <ArrowUpRightIcon
                    size={12}
                    className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                  />
                </h3>
                <p className="mt-1 text-xs text-muted-foreground">More projects and experiments</p>
              </div>
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ─── Tech Stack ───────────────────────────────────────────────── */

const techStack = [
  {
    icon: CpuIcon,
    label: "GPUI",
    description: "Zed's GPU-accelerated UI framework",
  },
  {
    icon: CodeIcon,
    label: "Rust",
    description: "Memory-safe, blazing fast",
  },
  {
    icon: DatabaseIcon,
    label: "Query Cache",
    description: "TTL, SWR, LatestWins policies",
  },
  {
    icon: StackIcon,
    label: "3-Layer Arch",
    description: "Core, Client, Hook separation",
  },
  {
    icon: ShieldCheckIcon,
    label: "Type Safe",
    description: "Compile-time guarantees",
  },
  {
    icon: PackageIcon,
    label: "Crate.io",
    description: "Published Rust package",
  },
];

function TechStackSection() {
  return (
    <section className="border-t border-border py-20 sm:py-28">
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
              className="group flex flex-col items-center ring-1 ring-foreground/8 bg-card p-5 text-center transition-all duration-200 hover:ring-primary/25"
            >
              <div className="mb-3 flex h-12 w-12 items-center justify-center bg-primary/10 text-primary transition-colors group-hover:bg-primary/15">
                <tech.icon size={24} />
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
    <section className="border-t border-border bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-3xl">
          <Card className="border-primary/10">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
                  <HeartIcon size={20} />
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
                    href="https://github.com/freeoxide/gpui-query"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <GithubLogoIcon size={16} className="mr-1.5" />
                    Star on GitHub
                  </a>
                </Button>
                <Button variant="outline" size="sm" asChild>
                  <a
                    href="https://github.com/freeoxide/gpui-query/issues"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Report an Issue
                    <ArrowUpRightIcon size={12} className="ml-1.5" />
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
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
  description: string;
}) {
  return (
    <div className="group flex items-start gap-4 ring-1 ring-foreground/8 bg-card p-5 transition-all duration-200 hover:ring-primary/25">
      <div className="flex h-10 w-10 shrink-0 items-center justify-center bg-primary/10 text-primary transition-colors group-hover:bg-primary/15">
        <Icon size={20} />
      </div>
      <div className="min-w-0">
        <h3 className="flex items-center gap-1.5 font-semibold">
          {label}
          <ArrowUpRightIcon
            size={12}
            className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
          />
        </h3>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

function LinksSection() {
  return (
    <section className="border-t border-border py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="text-center">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">Explore More</h2>
          <p className="mx-auto mt-4 max-w-2xl text-muted-foreground">
            Dive deeper into gpui-query with these resources.
          </p>
        </div>

        <div className="mx-auto mt-14 grid max-w-3xl gap-4 sm:grid-cols-2">
          <a
            href="https://github.com/freeoxide/gpui-query"
            target="_blank"
            rel="noopener noreferrer"
          >
            <LinkCard
              icon={GithubLogoIcon}
              label="GitHub"
              description="Source code, issues, and releases"
            />
          </a>

          <a href="/docs/">
            <LinkCard
              icon={BookOpenIcon}
              label="Documentation"
              description="Guides, API reference, and examples"
            />
          </a>

          <Link to="/blog">
            <LinkCard
              icon={NotePencilIcon}
              label="Blog"
              description="Announcements and deep dives"
            />
          </Link>

          <Link to="/changelog">
            <LinkCard
              icon={FileTextIcon}
              label="Changelog"
              description="Release history and breaking changes"
            />
          </Link>
        </div>
      </div>
    </section>
  );
}
