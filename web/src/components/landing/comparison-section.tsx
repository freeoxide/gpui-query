import { CheckIcon, XIcon } from "@phosphor-icons/react";
import { CornerMarks, SectionHeading } from "./decor";

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
  { label: "cx.spawn()", highlight: false },
  { label: "Raw Futures", highlight: false },
];

export function ComparisonSection() {
  return (
    <section className="border-y border-border bg-muted/30 py-20 sm:py-28">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <SectionHeading
          eyebrow="comparison"
          title="Why gpui-query?"
          description="See how gpui-query compares to hand-rolling async state in GPUI."
        />

        <div className="relative mx-auto mt-14 max-w-3xl" data-reveal data-reveal-delay="2">
          <CornerMarks />
          <div className="overflow-x-auto border border-border bg-background">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="py-4 pr-4 pl-5 text-left font-mono text-[11px] tracking-[0.2em] uppercase text-muted-foreground">
                    Feature
                  </th>
                  {columns.map((col) => (
                    <th
                      key={col.label}
                      className={`px-4 py-4 text-center font-mono text-[11px] tracking-[0.15em] uppercase ${
                        col.highlight ? "bg-primary/5 text-primary" : "text-muted-foreground"
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
                    className="border-b border-border transition-colors last:border-0 hover:bg-muted/40"
                  >
                    <td className="py-3.5 pr-4 pl-5 text-sm font-medium text-foreground">
                      {feature}
                    </td>
                    <td className="bg-primary/5 px-4 py-3.5 text-center">
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
      </div>
    </section>
  );
}

function SupportCell({ value }: { value: boolean }) {
  if (value) {
    return (
      <span className="inline-flex h-6 w-6 items-center justify-center bg-primary/10 text-primary ring-1 ring-primary/25">
        <CheckIcon size={13} weight="bold" />
      </span>
    );
  }
  return (
    <span className="inline-flex h-6 w-6 items-center justify-center text-muted-foreground/40 ring-1 ring-border">
      <XIcon size={13} />
    </span>
  );
}
