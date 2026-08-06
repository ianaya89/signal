import { cn } from "@/lib/utils";

export interface PaneTab {
  /** Also the visible label — these panes name themselves. */
  key: string;
  /** CSS colour expression for this channel, e.g. `var(--sec-library)`. */
  tone: string;
  blurb: string;
}

/**
 * A pane split into channels, styled like a rack strip: each tab owns a hue,
 * the active one wears it as a cap and the rest keep it dimmed so the strip
 * reads as a set rather than one accent among greys.
 *
 * The active channel's hue is published as `--section`, so anything rendered
 * inside can pick it up (focus rings, section ticks, hover states) without
 * being told which pane it is in.
 *
 * Selection is the caller's business — settings remembers it, the system pane
 * derives it from the route — so this owns no state.
 */
export function TabbedPane({
  tabs,
  active,
  onSelect,
  label,
  idPrefix,
  highlight,
  children,
}: {
  tabs: readonly PaneTab[];
  active: string;
  onSelect: (key: string) => void;
  /** Accessible name for the tablist. */
  label: string;
  /** Namespaces the tab/panel ids, so two panes can coexist. */
  idPrefix: string;
  /** Optional decorator for body copy — used to pick out key terms. */
  highlight?: (text: string) => React.ReactNode;
  children: React.ReactNode;
}) {
  const current = tabs.find((t) => t.key === active) ?? tabs[0];
  const show = highlight ?? ((text: string) => text);

  // a tablist is expected to move with the arrow keys, not just Tab
  const onKeyDown = (e: React.KeyboardEvent) => {
    const delta = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (delta === 0) return;
    e.preventDefault();
    const index = tabs.findIndex((t) => t.key === active);
    const next = tabs[(index + delta + tabs.length) % tabs.length];
    onSelect(next.key);
    document.getElementById(`${idPrefix}-tab-${next.key}`)?.focus();
  };

  return (
    <div
      className="flex h-full flex-col"
      style={{ "--section": current.tone } as React.CSSProperties}
    >
      <div
        role="tablist"
        aria-label={label}
        onKeyDown={onKeyDown}
        className="flex h-8 shrink-0 items-stretch gap-px overflow-x-auto border-b border-subtle bg-base/40 px-2 text-[10px]"
      >
        {tabs.map(({ key, tone }) => {
          const on = key === current.key;
          return (
            <button
              key={key}
              type="button"
              role="tab"
              id={`${idPrefix}-tab-${key}`}
              aria-selected={on}
              aria-controls={`${idPrefix}-panel-${key}`}
              tabIndex={on ? 0 : -1}
              onClick={() => onSelect(key)}
              style={{ "--tone": tone } as React.CSSProperties}
              className={cn(
                // the 2px cap is the channel colour; it dims rather than
                // disappears when inactive, so the strip reads as a set
                "shrink-0 border-t-2 px-2 transition-colors",
                on
                  ? "border-t-[color:var(--tone)] bg-raised text-[color:var(--tone)]"
                  : "border-t-[color-mix(in_srgb,var(--tone)_25%,transparent)] text-muted hover:border-t-[color-mix(in_srgb,var(--tone)_65%,transparent)] hover:text-secondary",
              )}
            >
              {key}
            </button>
          );
        })}
      </div>

      <header className="shrink-0 px-4 pb-2 pt-3">
        <h2 className="text-[13px] uppercase tracking-[0.2em] text-[color:var(--section)]">
          {show(current.key)}
        </h2>
        <p className="mt-1 text-[11px] text-secondary">{show(current.blurb)}</p>
        <div className="rule-fade mt-2" />
      </header>

      <div
        role="tabpanel"
        id={`${idPrefix}-panel-${current.key}`}
        aria-labelledby={`${idPrefix}-tab-${current.key}`}
        // scrolls here: the panes below are plain growing blocks that used to
        // rely on the outer Pane's scroller, which this sits inside of
        className="min-h-0 flex-1 overflow-auto bg-[linear-gradient(to_bottom,color-mix(in_srgb,var(--section)_7%,transparent),transparent_10rem)]"
      >
        {children}
      </div>
    </div>
  );
}
