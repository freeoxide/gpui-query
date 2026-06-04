import { CheckIcon, CopyIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";
import { useState, useRef, type ReactNode } from "react";

interface CodeBlockProps {
  children: ReactNode;
  title?: string;
}

export function CodeBlock({ children, title }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const codeRef = useRef<HTMLDivElement>(null);

  const handleCopy = async () => {
    const text = codeRef.current?.textContent ?? "";
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="group shadow-sm">
      {title && (
        <div className="flex items-center justify-between rounded-t-lg bg-muted/80 px-4 py-2 text-xs text-muted-foreground">
          <span className="font-mono">{title}</span>
        </div>
      )}
      <div
        className={[
          "relative overflow-x-auto font-mono text-sm",
          "bg-[var(--color-code-background)] border border-border",
          title ? "rounded-b-lg border-t-0" : "rounded-lg",
        ].join(" ")}
      >
        <Button
          variant="ghost"
          size="icon"
          className="absolute right-2 top-2 z-10 h-7 w-7 opacity-0 transition-opacity group-hover:opacity-100"
          onClick={handleCopy}
          aria-label={copied ? "Copied" : "Copy code"}
        >
          {copied ? (
            <CheckIcon size={14} className="text-green-500" />
          ) : (
            <CopyIcon size={14} />
          )}
        </Button>
        <div ref={codeRef}>{children}</div>
      </div>
      {copied && (
        <span className="pointer-events-none absolute right-2 top-2 rounded bg-primary px-2 py-0.5 text-xs text-primary-foreground shadow-sm transition-opacity">
          Copied!
        </span>
      )}
    </div>
  );
}
