import { CheckIcon, CopyIcon } from "@phosphor-icons/react";
import { useState } from "react";

/** Crosshair "+" registration marks pinned to the four corners of a relative parent. */
export function CornerMarks() {
  const positions = [
    "-top-1 -left-1",
    "-top-1 -right-1",
    "-bottom-1 -left-1",
    "-bottom-1 -right-1",
  ];
  return (
    <>
      {positions.map((pos) => (
        <svg
          key={pos}
          aria-hidden="true"
          width="9"
          height="9"
          viewBox="0 0 9 9"
          className={`pointer-events-none absolute ${pos} text-primary/60`}
        >
          <path d="M4.5 0v9M0 4.5h9" stroke="currentColor" strokeWidth="1" />
        </svg>
      ))}
    </>
  );
}

interface SectionHeadingProps {
  eyebrow: string;
  title: string;
  description?: string;
  align?: "left" | "center";
}

/** Shared section header: `// eyebrow` comment + Oxanium display title. */
export function SectionHeading({
  eyebrow,
  title,
  description,
  align = "left",
}: SectionHeadingProps) {
  const centered = align === "center";
  return (
    <div data-reveal className={centered ? "text-center" : ""}>
      <p className="font-mono text-xs tracking-[0.25em] uppercase text-primary">
        <span className="text-primary/50">{"// "}</span>
        {eyebrow}
      </p>
      <h2 className="mt-3 font-display text-3xl font-bold tracking-tight sm:text-4xl">{title}</h2>
      {description && (
        <p className={`mt-4 max-w-2xl text-muted-foreground ${centered ? "mx-auto" : ""}`}>
          {description}
        </p>
      )}
    </div>
  );
}

/** Terminal-style one-liner with a copy affordance. */
export function CommandLine({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative inline-flex items-center gap-3 border border-border bg-card py-3 pr-3 pl-4 font-mono text-sm shadow-sm">
      <CornerMarks />
      <span aria-hidden="true" className="text-primary select-none">
        $
      </span>
      <span className="text-foreground">{command}</span>
      <button
        type="button"
        onClick={handleCopy}
        aria-label={copied ? "Copied" : "Copy command"}
        className="ml-2 flex h-7 w-7 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-primary"
      >
        {copied ? (
          <CheckIcon size={14} weight="bold" className="text-primary" />
        ) : (
          <CopyIcon size={14} />
        )}
      </button>
    </div>
  );
}
