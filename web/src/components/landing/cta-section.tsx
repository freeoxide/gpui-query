import { ArrowRightIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";

export function CtaSection() {
  return (
    <section className="border-t border-border py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div
          className="mx-auto max-w-2xl text-center"
          data-reveal
        >
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Start building with gpui-query
          </h2>
          <p className="mx-auto mt-4 max-w-md text-muted-foreground">
            Read the documentation or star the repo.
          </p>

          <div className="mt-10 flex flex-col items-center gap-4">
            <Button size="lg" asChild>
              <a href="/docs/">
                Get Started
                <ArrowRightIcon size={16} className="ml-1" />
              </a>
            </Button>

            <a
              href="https://github.com/hmziqrs/gpui-query"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-primary"
            >
              View on GitHub
              <ArrowRightIcon size={12} />
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
