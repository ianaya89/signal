import type { ReactNode } from "react";
import { createPortal } from "react-dom";

import { cn } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";

/**
 * View-level controls — sorts, filters, the one action a view exists for —
 * render into the main pane's title bar instead of a strip of their own.
 *
 * A per-view toolbar cost a row of vertical space in every list, repeated the
 * `[ title ]` bar's job one line lower, and left sticky section headers with a
 * band of scrolled content showing above them (a sticky child cannot rise past
 * its scroll container's padding box). One bar, title left, controls right.
 */
export function PaneActions({ children }: { children: ReactNode }) {
  const slot = useUiStore((s) => s.mainHeaderSlot);
  return slot ? createPortal(children, slot) : null;
}

/** Sort switch: the sorts a view offers, current one lit. */
export function PaneSort<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: readonly { key: T; label: string }[];
  onChange: (key: T) => void;
}) {
  return (
    <span className="flex items-center gap-0.5 text-[10px]">
      <span className="text-muted">sort:</span>
      {options.map(({ key, label }) => (
        <button
          key={key}
          type="button"
          onClick={() => onChange(key)}
          className={cn(
            "px-1 py-px",
            value === key
              ? "bg-raised text-accent"
              : "text-muted hover:text-secondary",
          )}
        >
          {label}
        </button>
      ))}
    </span>
  );
}

/**
 * A view's actions, sized for the title bar.
 *
 * `tone="primary"` is the one that starts playback — tinted at rest so it
 * reads as the action before you hover it; everything else is an outline. At
 * 10px the two are still told apart by fill rather than by weight.
 */
export function PaneAction({
  tone = "default",
  className,
  children,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  tone?: "default" | "primary";
}) {
  return (
    <button
      type="button"
      {...props}
      className={cn(
        "flex shrink-0 items-center gap-1 border px-1.5 py-px text-[10px] transition-colors disabled:cursor-not-allowed disabled:opacity-40",
        tone === "primary"
          ? "border-accent-dim bg-accent-dim/25 text-accent hover:border-focus hover:bg-accent-dim/45"
          : "border-subtle bg-raised text-secondary hover:border-focus hover:text-accent",
        className,
      )}
    >
      {children}
    </button>
  );
}

/** Vertical hairline between groups of controls in the title bar. */
export function PaneActionsDivider() {
  return <span aria-hidden className="h-3 w-px shrink-0 bg-subtle" />;
}
