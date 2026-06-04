import { Link } from "@tanstack/react-router";
import { ChevronLeft, ChevronRight } from "lucide-react";

interface PaginationLink {
  slug: string;
  title: string;
}

interface DocsPaginationProps {
  prev?: PaginationLink;
  next?: PaginationLink;
}

export function DocsPagination({ prev, next }: DocsPaginationProps) {
  return (
    <div className="mt-12 flex items-center justify-between border-t pt-6">
      {prev ? (
        <Link
          to="/docs/$slug"
          params={{ slug: prev.slug }}
          className="group flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          <ChevronLeft className="h-4 w-4 transition-transform group-hover:-translate-x-1" />
          <div className="flex flex-col">
            <span className="text-xs">Previous</span>
            <span className="font-medium">{prev.title}</span>
          </div>
        </Link>
      ) : (
        <div />
      )}
      {next ? (
        <Link
          to="/docs/$slug"
          params={{ slug: next.slug }}
          className="group flex items-center gap-2 text-right text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          <div className="flex flex-col">
            <span className="text-xs">Next</span>
            <span className="font-medium">{next.title}</span>
          </div>
          <ChevronRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
        </Link>
      ) : (
        <div />
      )}
    </div>
  );
}
