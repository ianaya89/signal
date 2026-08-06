/**
 * Shared control styling.
 *
 * These strings had been copied by hand across settings, doctor, playlists and
 * remote — byte-identical in some places, drifted by a padding step in others.
 * One definition is what makes a restyle a single edit instead of a search.
 *
 * The accent resolves to `--section` when the control sits inside a pane that
 * publishes a channel hue (see `TabbedPane`), and falls back to the global
 * accent everywhere else — so a button looks native to whichever pane it lands
 * in without being told which one that is.
 *
 * Written out in full, deliberately. Tailwind extracts class names by scanning
 * source text, so it cannot see through a template literal: building these
 * from shared `${ACCENT}` fragments compiles to no CSS at all, silently, and
 * the build still succeeds. Keep every class name literal here.
 */

/** The default weight. Most buttons are this — if everything is emphasised,
 *  nothing is. */
export const BTN =
  "shrink-0 border border-subtle bg-raised px-2 py-0.5 text-[11px] text-secondary transition-colors hover:border-[color:var(--section,var(--border-focus))] hover:text-[color:var(--section,var(--accent))]";

/**
 * The one action a view exists for: submit the form, start the server, play
 * the album. Filled rather than outlined, because in a flat square-cornered UI
 * weight is the only hierarchy available — there are no shadows or radii to
 * lean on. At most one per context, or it stops meaning anything.
 *
 * Fill and label are their own tokens rather than `--accent` and `--bg-base`:
 * that pairing measured 3.3:1 on the manila theme, under AA. The split lets
 * light use a deeper wax and a near-white label (4.9:1) while dark keeps
 * periwinkle under near-black (5.9:1).
 *
 * Note this one does NOT pick up `--section`, unlike every other control here.
 * Two reasons, and they agree: a primary that took the pane's hue would blend
 * into the pane instead of standing out from it, and half the channel hues are
 * too light to carry a label at 4.5:1 on the manila theme — amber measured
 * 3.7:1. One fill, always legible, always reads as "the action".
 */
export const BTN_PRIMARY =
  "shrink-0 border border-[color:var(--accent-fill)] bg-[color:var(--accent-fill)] px-2 py-0.5 text-[11px] font-semibold text-[color:var(--on-accent)] transition-opacity hover:opacity-85 disabled:cursor-not-allowed disabled:opacity-40";

/** Destructive actions read as destructive at rest, not only on hover. */
export const BTN_DANGER =
  "shrink-0 border border-subtle bg-raised px-2 py-0.5 text-[11px] text-error/80 transition-colors hover:border-error hover:text-error";

export const INPUT =
  "border border-subtle bg-base/60 px-2 py-0.5 text-[11px] text-primary outline-none focus:border-[color:var(--section,var(--border-focus))]";

/** Same as `INPUT` at the larger step used by dialogs and editors. */
export const INPUT_LG =
  "border border-subtle bg-base/60 px-2 py-1 text-[12px] text-primary outline-none focus:border-[color:var(--section,var(--border-focus))]";
