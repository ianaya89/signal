import type { SortKey, TrackSort } from "@/hooks/useTrackSort";
import { cn } from "@/lib/utils";

const COLUMNS: { key: SortKey; label: string; className: string }[] = [
  { key: "default", label: "#", className: "w-10 pr-2 text-right" },
  { key: "title", label: "title", className: "pr-2 text-left" },
  { key: "codec", label: "codec", className: "w-32 pr-2 text-left" },
  { key: "rating", label: "♥", className: "w-12 pr-1 text-right" },
  { key: "duration", label: "dur", className: "w-12 pr-3 text-right" },
];

export function TrackTableHeader({ sort }: { sort: TrackSort }) {
  return (
    <thead>
      <tr className="h-6 border-b border-subtle">
        {COLUMNS.map((col) => (
          <th key={col.label} className={cn("font-normal", col.className)}>
            <button
              type="button"
              onClick={() => sort.toggle(col.key)}
              className={cn(
                "text-[10px]",
                sort.key === col.key && col.key !== "default"
                  ? "text-accent"
                  : "text-muted hover:text-secondary",
              )}
            >
              {col.label}
              {sort.key === col.key && col.key !== "default"
                ? sort.desc
                  ? " ▾"
                  : " ▴"
                : ""}
            </button>
          </th>
        ))}
        <th className="w-8" />
      </tr>
    </thead>
  );
}
