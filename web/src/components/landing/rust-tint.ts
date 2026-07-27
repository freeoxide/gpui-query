// Minimal Rust syntax tinter for landing code snippets. Pure logic module (no
// markup) — kept as .ts so it can be imported as a named export by rust-code.astro.
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

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
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

// Two accents only (keywords/strings on primary, comments muted), matching the
// hero style. Returns an escaped HTML string to inject via set:html.
export function tintRust(line: string): string {
  return tokenizeLine(line)
    .map(({ part }) => {
      const esc = escapeHtml(part);
      if (part.startsWith("//")) return `<span class="text-muted-foreground">${esc}</span>`;
      if (part.startsWith('"')) return `<span class="text-primary/70">${esc}</span>`;
      if (KEYWORDS.has(part)) return `<span class="text-primary">${esc}</span>`;
      return `<span>${esc}</span>`;
    })
    .join("");
}
