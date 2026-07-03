import type { ReactNode } from "react";

/**
 * Minimal Rust tinter for landing snippets — two accents only
 * (keywords/strings on primary, comments muted), matching the hero style.
 */

const KEYWORDS = new Set([
  "let",
  "use",
  "fn",
  "impl",
  "struct",
  "enum",
  "match",
  "if",
  "else",
  "return",
  "pub",
  "mut",
  "for",
  "in",
  "async",
  "move",
  "await",
  "Some",
  "None",
  "Ok",
  "Err",
]);

const TOKEN = /(\/\/.*$|"[^"]*"|\b[A-Za-z_][A-Za-z0-9_]*\b)/g;

export function tintRust(line: string, lineKey: string): ReactNode[] {
  return tintLine(line, lineKey);
}

function tintLine(line: string, lineKey: string): ReactNode[] {
  return line.split(TOKEN).map((part, i) => {
    const key = `${lineKey}-${i}`;
    if (part.startsWith("//")) {
      return (
        <span key={key} className="text-muted-foreground">
          {part}
        </span>
      );
    }
    if (part.startsWith('"')) {
      return (
        <span key={key} className="text-primary/70">
          {part}
        </span>
      );
    }
    if (KEYWORDS.has(part)) {
      return (
        <span key={key} className="text-primary">
          {part}
        </span>
      );
    }
    return <span key={key}>{part}</span>;
  });
}

interface RustCodeProps {
  code: string;
  lineNumbers?: boolean;
  className?: string;
}

export function RustCode({ code, lineNumbers = false, className = "" }: RustCodeProps) {
  const lines = code.split("\n");
  return (
    <pre
      className={`overflow-x-auto p-5 font-mono text-[13px] leading-6 text-foreground/85 ${className}`}
    >
      <code>
        {lines.map((line, i) => (
          <span key={`l-${String(i)}`} className="block">
            {lineNumbers && (
              <span className="mr-4 inline-block w-5 text-right text-muted-foreground/40 select-none">
                {i + 1}
              </span>
            )}
            {tintLine(line, `l-${String(i)}`)}
          </span>
        ))}
      </code>
    </pre>
  );
}
