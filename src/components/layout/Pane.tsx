import type { ReactNode } from "react";

import { cn } from "@/lib/utils";
import { useUiStore, type PaneId } from "@/stores/uiStore";

interface PaneProps {
  id: PaneId;
  title: string;
  className?: string;
  style?: React.CSSProperties;
  children: ReactNode;
}

export function Pane({ id, title, className, style, children }: PaneProps) {
  const focused = useUiStore((s) => s.focusedPane === id);
  const focusPane = useUiStore((s) => s.focusPane);

  return (
    <section
      onMouseDown={() => focusPane(id)}
      style={style}
      className={cn(
        "flex min-h-0 flex-col overflow-hidden border bg-surface",
        focused ? "border-focus" : "border-subtle",
        className,
      )}
    >
      <header className="flex h-6 shrink-0 items-center justify-between border-b border-subtle px-2">
        <span className={cn("text-[11px]", focused ? "text-accent" : "text-muted")}>
          [ {title} ]
        </span>
        {focused && <span className="text-[10px] text-accent">▮</span>}
      </header>
      <div className="min-h-0 flex-1 overflow-auto">{children}</div>
    </section>
  );
}
