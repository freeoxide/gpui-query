import type { ReactNode } from "react";

interface LegalPageProps {
  title: string;
  description: string;
  updatedAt: string;
  children: ReactNode;
}

interface LegalSectionProps {
  title: string;
  children: ReactNode;
}

export function LegalPage({ title, description, updatedAt, children }: LegalPageProps) {
  return (
    <div className="mx-auto max-w-3xl px-4 py-10 sm:px-6 sm:py-16 lg:px-8 lg:py-20">
      <header className="border-b border-border pb-8">
        <p className="font-mono text-xs font-medium uppercase tracking-[0.18em] text-primary">
          Website legal
        </p>
        <h1 className="mt-4 text-4xl font-bold tracking-tight text-foreground sm:text-5xl">
          {title}
        </h1>
        <p className="mt-5 max-w-2xl text-base leading-7 text-muted-foreground">{description}</p>
        <p className="mt-6 font-mono text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
          Last updated <span className="ml-2 font-sans text-sm normal-case tracking-normal text-foreground">{updatedAt}</span>
        </p>
      </header>

      <article className="mt-8 border border-border bg-card shadow-sm">
        <div className="border-b border-border bg-muted/30 px-5 py-5 sm:px-8">
          <p className="text-sm leading-6 text-muted-foreground">
            These pages are intentionally plain-language and scoped to this website. They do not
            change the license terms for the gpui-query source code.
          </p>
        </div>
        <div className="divide-y divide-border">{children}</div>
      </article>
    </div>
  );
}

export function LegalSection({ title, children }: LegalSectionProps) {
  return (
    <section className="px-5 py-7 sm:px-8 sm:py-8">
      <h2 className="text-lg font-semibold tracking-tight text-foreground sm:text-xl">{title}</h2>
      <div className="mt-3 space-y-4 text-[15px] leading-7 text-muted-foreground [&_a]:font-medium [&_a]:text-primary [&_a]:underline [&_a]:decoration-primary/30 [&_a]:underline-offset-4 [&_a:hover]:decoration-primary [&_code]:border [&_code]:border-border [&_code]:bg-muted [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.9em] [&_code]:text-foreground [&_strong]:font-semibold [&_strong]:text-foreground">
        {children}
      </div>
    </section>
  );
}
