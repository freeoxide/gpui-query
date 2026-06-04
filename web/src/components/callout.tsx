import { Info, AlertTriangle, Lightbulb, MessageSquare } from "lucide-react";
import { cn } from "#/lib/utils";
import type { ReactNode } from "react";

type CalloutType = "info" | "warning" | "tip" | "note";

interface CalloutProps {
  type?: CalloutType;
  children: ReactNode;
}

const calloutConfig: Record<CalloutType, { icon: ReactNode; className: string }> = {
  info: {
    icon: <Info className="h-5 w-5" />,
    className: "border-blue-500/50 bg-blue-500/10 text-blue-700 dark:text-blue-400",
  },
  warning: {
    icon: <AlertTriangle className="h-5 w-5" />,
    className: "border-yellow-500/50 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400",
  },
  tip: {
    icon: <Lightbulb className="h-5 w-5" />,
    className: "border-green-500/50 bg-green-500/10 text-green-700 dark:text-green-400",
  },
  note: {
    icon: <MessageSquare className="h-5 w-5" />,
    className: "border-muted-foreground/50 bg-muted text-foreground",
  },
};

export function Callout({ type = "note", children }: CalloutProps) {
  const config = calloutConfig[type] ?? calloutConfig.note;

  return (
    <div
      className={cn("my-4 flex gap-3 rounded-lg border p-4", config.className)}
      role={type === "warning" ? "alert" : "note"}
    >
      <div className="mt-0.5 shrink-0">{config.icon}</div>
      <div className="min-w-0 flex-1 text-sm">{children}</div>
    </div>
  );
}
