import type { ReactNode } from "react";

/**
 * Minimal Rust tinter for landing snippets - two accents only
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
  return tokenizeLine(line).map(({ part, offset }) => {
    const key = `${lineKey}-${offset}`;
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

function tokenizeLine(line: string): Array<{ part: string; offset: number }> {
  const parts: Array<{ part: string; offset: number }> = [];
  let lastIndex = 0;

  for (const match of line.matchAll(TOKEN)) {
    const offset = match.index ?? 0;
    if (offset > lastIndex) {
      parts.push({ part: line.slice(lastIndex, offset), offset: lastIndex });
    }
    parts.push({ part: match[0], offset });
    lastIndex = offset + match[0].length;
  }

  if (lastIndex < line.length) {
    parts.push({ part: line.slice(lastIndex), offset: lastIndex });
  }

  return parts;
}
