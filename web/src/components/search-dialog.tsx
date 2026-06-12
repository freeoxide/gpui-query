import { useEffect, useRef, useState, useCallback } from "react";
import { Search, X, FileText, ArrowUp, ArrowDown, Command, FileSearch } from "lucide-react";

interface SearchResult {
  url: string;
  title: string;
  excerpt: string;
  score: number;
}

const backdropAnimation = {
  animation: "searchBackdropIn 150ms ease-out forwards",
};

const dialogAnimation = {
  animation: "searchDialogIn 200ms ease-out forwards",
};

const keyframeStyles = `
@keyframes searchBackdropIn {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes searchDialogIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
`;

function KeyboardHint({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex items-center gap-0.5 rounded border border-border/60 bg-muted/60 px-1.5 py-0.5 font-mono text-[10px] font-medium text-muted-foreground shadow-sm">
      {children}
    </kbd>
  );
}

export function SearchDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [pagefind, setPagefind] = useState<any>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  // Lazy-load Pagefind on first open
  useEffect(() => {
    if (!open || pagefind) return;
    async function load() {
      try {
        // Dynamic non-literal path to bypass Vite import analysis
        const pagefindPath = ["/pagefind", "pagefind.js"].join("/");
        // @ts-expect-error Pagefind is loaded at runtime
        const pf = await import(/* @vite-ignore */ pagefindPath);
        await pf.init();
        setPagefind(pf);
      } catch {
        // Pagefind not available in dev mode
        console.warn("Pagefind not available. Run build first.");
      }
    }
    void load();
  }, [open, pagefind]);

  // Focus input on open
  useEffect(() => {
    if (open) {
      setQuery("");
      setResults([]);
      setSelectedIndex(-1);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  // Search
  useEffect(() => {
    if (!pagefind || !query.trim()) {
      setResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const search = await pagefind.search(query);
        const items: SearchResult[] = await Promise.all(
          search.results.slice(0, 10).map(async (r: any) => {
            const data = await r.data();
            return {
              url: data.url,
              title: data.meta?.title ?? data.url,
              excerpt: data.excerpt ?? "",
              score: r.score,
            };
          }),
        );
        setResults(items);
        setSelectedIndex(0);
      } catch {
        setResults([]);
      }
    }, 200);

    return () => clearTimeout(timer);
  }, [query, pagefind]);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && selectedIndex >= 0 && results[selectedIndex]) {
        e.preventDefault();
        window.location.href = results[selectedIndex].url;
        onClose();
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [results, selectedIndex, onClose],
  );

  // Scroll selected item into view
  useEffect(() => {
    if (selectedIndex >= 0 && listRef.current) {
      const item = listRef.current.children[selectedIndex] as HTMLElement;
      item?.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
      onKeyDown={handleKeyDown}
    >
      <style>{keyframeStyles}</style>

      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/50 backdrop-blur-sm"
        style={backdropAnimation}
        onClick={onClose}
      />

      {/* Dialog */}
      <div
        className="relative z-10 w-full max-w-lg overflow-hidden rounded-xl border bg-background shadow-2xl"
        style={dialogAnimation}
      >
        {/* Gradient top border */}
        <div
          className="h-0.5 w-full"
          style={{
            background:
              "linear-gradient(to right, hsl(var(--primary)), hsl(var(--primary) / 0.1), transparent)",
          }}
        />

        {/* Search input */}
        <div className="flex items-center border-b px-4">
          <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            placeholder="Search documentation..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="flex-1 bg-transparent px-3 py-3 text-sm outline-none placeholder:text-muted-foreground"
          />
          <kbd className="hidden items-center gap-1 rounded border bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground sm:flex">
            <Command className="h-2.5 w-2.5" />K
          </kbd>
          <button onClick={onClose} className="ml-2 rounded p-1 hover:bg-muted">
            <X className="h-3.5 w-3.5 text-muted-foreground" />
          </button>
        </div>

        {/* Results */}
        {results.length > 0 ? (
          <ul ref={listRef} className="max-h-80 overflow-y-auto p-2">
            {results.map((result, i) => (
              <li key={result.url}>
                <a
                  href={result.url}
                  onClick={(e) => {
                    e.preventDefault();
                    window.location.href = result.url;
                    onClose();
                  }}
                  className={`relative flex items-start gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors ${
                    i === selectedIndex ? "bg-accent text-accent-foreground" : "hover:bg-muted"
                  }`}
                  style={
                    i === selectedIndex
                      ? {
                          borderLeft: "2px solid hsl(var(--primary))",
                          paddingLeft: "10px",
                        }
                      : {
                          borderLeft: "2px solid transparent",
                          paddingLeft: "10px",
                        }
                  }
                >
                  <FileText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0 flex-1">
                    <div className="font-medium truncate">{result.title}</div>
                    <div
                      className="mt-0.5 text-xs text-muted-foreground line-clamp-2"
                      dangerouslySetInnerHTML={{ __html: result.excerpt }}
                    />
                  </div>
                </a>
              </li>
            ))}
          </ul>
        ) : query && pagefind ? (
          <div className="flex flex-col items-center justify-center gap-2 px-4 py-10 text-center">
            <FileSearch className="h-8 w-8 text-muted-foreground/40" />
            <p className="text-sm text-muted-foreground">
              No results found for &ldquo;{query}&rdquo;
            </p>
          </div>
        ) : !pagefind ? (
          <div className="flex flex-col items-center justify-center gap-2 px-4 py-10 text-center">
            <Search className="h-8 w-8 text-muted-foreground/40" />
            <p className="text-sm text-muted-foreground">
              Search requires a build. Run{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-xs">vp build</code> first.
            </p>
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center gap-2 px-4 py-10 text-center">
            <Search className="h-8 w-8 text-muted-foreground/40" />
            <p className="text-sm text-muted-foreground">Type to search documentation...</p>
          </div>
        )}

        {/* Footer */}
        <div className="flex items-center justify-between border-t px-4 py-2.5">
          {results.length > 0 ? (
            <div className="flex items-center gap-4">
              <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <KeyboardHint>
                  <ArrowUp className="h-2.5 w-2.5" />
                  <ArrowDown className="h-2.5 w-2.5" />
                </KeyboardHint>
                Navigate
              </span>
              <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <KeyboardHint>Enter</KeyboardHint>
                Open
              </span>
              <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <KeyboardHint>Esc</KeyboardHint>
                Close
              </span>
            </div>
          ) : (
            <div />
          )}
          <span className="text-[10px] text-muted-foreground/50">Powered by Pagefind</span>
        </div>
      </div>
    </div>
  );
}
