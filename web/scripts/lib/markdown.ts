/**
 * Markdown helpers shared by the alt-format generators.
 *
 * The site ships every page three ways: the rendered HTML, a clean `.md`
 * (lean structured markdown for agents that parse it), and a `.txt` (the
 * markdown flattened to running plain text — the absolute token floor). Both
 * alternates derive from one markdown string, so content is never duplicated.
 */

export interface PageParts {
  title: string;
  description?: string;
  /** Markdown body without frontmatter or an H1 (the title is prepended). */
  markdown: string;
}

/**
 * Assemble a page's markdown: a single H1, an optional one-line description,
 * then the body. Kept intentionally lean (no YAML frontmatter) to minimize
 * tokens — the title is the only metadata an agent reliably needs.
 */
export function pageMarkdown(page: PageParts): string {
  const lines = [`# ${page.title}`, ""];
  if (page.description) lines.push(page.description, "");
  lines.push(page.markdown.trim());
  return `${lines.join("\n")}\n`;
}

// Control-char sentinels built at runtime from plain ASCII so the source never
// holds a literal NUL byte. They guard inline code and fenced blocks while the
// emphasis/link/HTML transforms run over the surrounding prose.
const NUL = String.fromCharCode(0);
const US = String.fromCharCode(1);

/**
 * Flatten a GFM table row to space-separated cells (pipes dropped). Used only
 * by the .txt path so raw markdown tables don't leak; the .md path keeps them.
 */
function flattenTableRow(row: string): string {
  return row
    .replace(/^\s*\|/, "")
    .replace(/\|\s*$/, "")
    .split("|")
    .map((c) => c.trim())
    .join("  ");
}

/**
 * Flatten markdown to running plain text for `.txt` alternates.
 *
 * Code is sacred: fenced blocks AND inline code are extracted up front and
 * reinserted verbatim, so identifiers like `ttl_ms`, generics like
 * `Vec<User>`, and operators never get mangled. Underscore "emphasis" is
 * deliberately NOT stripped (would eat `begin_request`/`60_000`); single-`*`
 * emphasis is stripped only when genuinely flanking (no whitespace adjacent to
 * either delimiter) so arithmetic like `5 * 3` survives; the residual-HTML-tag
 * remover is restricted to real tag names so Rust generics like `Arc<AtomicBool>`
 * are never eaten.
 */
export function markdownToPlain(md: string): string {
  const lines = md.split(/\r?\n/);
  const out: string[] = [];
  const blocks: string[] = []; // fenced code block contents
  let inFence = false;
  let calloutFence = false; // opener was blockquote-wrapped (Starlight callout)
  let buf: string[] | null = null;
  let inTable = false;

  const flushBlock = () => {
    blocks.push((buf ?? []).join("\n"));
    out.push(`${US}${blocks.length - 1}${US}`);
    buf = null;
  };

  for (const originalLine of lines) {
    // Drop a leading blockquote marker FIRST: Starlight callouts (admonitions)
    // render the code fences they wrap as "> ```lang", so fence detection has
    // to see past the "> " or the whole callout leaks as raw markdown.
    const line = originalLine.replace(/^\s{0,3}>\s?/, "");

    // Fenced code blocks: collect the interior behind a placeholder.
    if (/^\s{0,3}(```|~~~)/.test(line)) {
      if (inFence) {
        flushBlock();
        inFence = false;
        calloutFence = false;
        inTable = false;
      } else {
        inFence = true;
        calloutFence = line !== originalLine; // opener was blockquote-wrapped
        buf = [];
      }
      continue;
    }
    if (inFence) {
      // Preserve normal code verbatim; strip "> " only from callout-wrapped fences.
      buf?.push(calloutFence ? line : originalLine);
      continue;
    }

    // ATX headings -> bare text.
    let l = line.replace(/^\s{0,3}#{1,6}\s+/, "");

    // Horizontal rule -> blank line (saves tokens, adds nothing readable).
    if (/^\s{0,3}([-*_])\1{2,}\s*$/.test(l)) {
      inTable = false;
      out.push("");
      continue;
    }

    // GFM tables: a delimiter row (|---|---|) right after a pipe row opens a
    // table. Flatten header + body rows and drop the delimiter so no raw pipe
    // syntax leaks into the .txt.
    const isDelimiter =
      l.includes("-") &&
      l.includes("|") &&
      /^\s*\|?[\s:|-]*-[\s:|-]*\|?\s*$/.test(l);
    if (!inTable && isDelimiter && out.length > 0 && out[out.length - 1].includes("|")) {
      out[out.length - 1] = flattenTableRow(out[out.length - 1]);
      inTable = true;
      continue;
    }
    if (inTable) {
      if (!l.includes("|")) {
        inTable = false; // blank or non-pipe line ends the table
        out.push(l);
        continue;
      }
      out.push(flattenTableRow(l));
      continue;
    }

    out.push(l);
  }
  if (inFence) flushBlock(); // unterminated fence — emit what we have

  let text = out.join("\n");

  const codes: string[] = [];
  text = text.replace(/`([^`]+)`/g, (_m, c: string) => {
    codes.push(c);
    return `${NUL}${codes.length - 1}${NUL}`;
  });

  // Prose only — code spans are sentinel-guarded above.
  // Images ![alt](url) -> alt.
  text = text.replace(/!\[([^\]]*)\]\([^)]+\)/g, "$1");
  // Links [text](url) -> "text (url)". For mailto:, the label is already the
  // address, so drop the redundant destination.
  text = text.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_m, label: string, url: string) => {
    const dest = url.trim();
    const lab = label.trim();
    if (/^mailto:/i.test(dest)) return lab;
    return lab === dest ? dest : `${lab} (${dest})`;
  });
  // Bold and strikethrough.
  text = text.replace(/\*\*([^*]+)\*\*/g, "$1");
  text = text.replace(/~~([^~\n]+)~~/g, "$1");
  // Italic (single *): only genuine flanking emphasis — the opening * must be
  // at start/whitespace/( and NOT followed by whitespace, the closing * must
  // NOT be preceded by whitespace. So `5 * 3` and `*const T` stay intact.
  text = text.replace(/(^|[\s(])\*(\S(?:[^*\n]*\S)?)\*(?=[\s.,;:!?)\]]|$)/g, "$1$2");
  // Residual raw HTML tags — RESTRICTED to real tag names (with a word
  // boundary after the name) so Rust generics like <AtomicBool>, <T, E>, and
  // <u8> are never mistaken for tags and stripped.
  text = text.replace(
    /<\/?(a|abbr|b|bdo|blockquote|br|cite|code|dd|div|dl|dt|em|h[1-6]|hr|i|img|kbd|li|mark|ol|p|pre|s|small|span|strong|sub|sup|table|tbody|td|tfoot|th|thead|tr|u|ul|var)\b[^>]*\/?>/gi,
    "",
  );

  // Restore code verbatim: inline spans first, then fenced blocks.
  text = text.replace(new RegExp(`${NUL}(\\d+)${NUL}`, "g"), (_m, i: string) => codes[Number(i)] ?? "");
  text = text.replace(new RegExp(`${US}(\\d+)${US}`, "g"), (_m, i: string) => blocks[Number(i)] ?? "");

  // Collapse 3+ blank lines left by removed fences/rules.
  text = text.replace(/\n{3,}/g, "\n\n").trim();
  return `${text}\n`;
}
