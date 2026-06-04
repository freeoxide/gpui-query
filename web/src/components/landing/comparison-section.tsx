import { CheckIcon, XIcon } from "@phosphor-icons/react";

type Support = boolean;
type Row = [string, Support, Support, Support];

const comparisonRows: Row[] = [
  ["Caching", true, false, false],
  ["Auto Retry", true, false, false],
  ["Deduplication", true, false, false],
  ["Cache Policies", true, false, false],
  ["DevTools", true, false, false],
  ["Persistence", true, false, false],
  ["Type Safety", true, true, true],
  ["No Setup Required", false, true, true],
  ["Zero Dependencies", false, false, true],
];

const columns = [
  { label: "gpui-query", highlight: true },
  { label: "Manual cx.spawn()", highlight: false },
  { label: "Raw Futures", highlight: false },
];

export function ComparisonSection() {
  return (
    <section className="border-t border-border bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div data-reveal>
          <p className="text-xs font-medium tracking-widest uppercase text-primary">
            Comparison
          </p>
          <h2 className="mt-2 text-3xl font-bold tracking-tight sm:text-4xl">
            Why gpui-query?
          </h2>
          <p className="mt-4 max-w-2xl text-muted-foreground">
            See how gpui-query compares to the alternatives for managing async
            state in GPUI.
          </p>
        </div>

        <div
          className="mx-auto mt-14 max-w-3xl overflow-hidden ring-1 ring-foreground/8"
          data-reveal
          data-reveal-delay="2"
        >
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/80">
                <th className="py-4 pr-4 pl-5 text-left text-sm font-semibold text-foreground">
                  Feature
                </th>
                {columns.map((col) => (
                  <th
                    key={col.label}
                    className={`px-4 py-4 text-center text-sm font-semibold ${
                      col.highlight
                        ? "text-primary"
                        : "text-muted-foreground"
                    }`}
                  >
                    {col.label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {comparisonRows.map(([feature, a, b, c]) => (
                <tr
                  key={feature}
                  className="border-b border-border last:border-0 transition-colors hover:bg-muted/40"
                >
                  <td className="py-3.5 pr-4 pl-5 text-sm font-medium text-foreground">
                    {feature}
                  </td>
                  <td className="bg-primary/[0.03] px-4 py-3.5 text-center">
                    <SupportCell value={a} />
                  </td>
                  <td className="px-4 py-3.5 text-center">
                    <SupportCell value={b} />
                  </td>
                  <td className="px-4 py-3.5 text-center">
                    <SupportCell value={c} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}

function SupportCell({ value }: { value: boolean }) {
  if (value) {
    return (
      <span className="inline-flex h-6 w-6 items-center justify-center ring-1 ring-primary/20 text-primary">
        <CheckIcon size={14} weight="bold" />
      </span>
    );
  }
  return (
    <span className="inline-flex h-6 w-6 items-center justify-center ring-1 ring-foreground/8 text-muted-foreground/50">
      <XIcon size={14} />
    </span>
  );
}
