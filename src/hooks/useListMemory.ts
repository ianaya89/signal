import { useEffect, useRef, useState } from "react";

/**
 * Where you were in a list, kept for the session.
 *
 * Routing unmounts a view, so albums → artists → back used to drop you at the
 * top with the cursor on row 0. An editor reopens a file on the line you left
 * it on; a library browser should reopen a list on the record you left it on.
 *
 * Module-level maps rather than a store: nothing renders off this state, it is
 * read once on mount and written as you move.
 */
const cursors = new Map<string, number>();
const offsets = new Map<string, number>();

/**
 * `useState` for a list cursor, seeded from the last visit.
 *
 * Pass `length` where the list can shrink between visits (unfavouriting a
 * track, say) so a remembered row past the end folds back onto the last one.
 * A length of 0 is read as "not loaded yet" and leaves the cursor alone —
 * clamping against an empty query would erase the position it is restoring.
 */
export function useListCursor(key: string, length?: number) {
  const [cursor, setCursor] = useState(() => cursors.get(key) ?? 0);
  const clamped = length ? Math.min(cursor, length - 1) : cursor;
  useEffect(() => {
    cursors.set(key, clamped);
  }, [key, clamped]);
  return [clamped, setCursor] as const;
}

/**
 * Restores the scroll offset of `ref` once `ready` (the query has rows, so the
 * container has height to scroll), then tracks it.
 */
export function useScrollMemory(
  key: string,
  ref: React.RefObject<HTMLElement | null>,
  ready: boolean,
) {
  const restored = useRef(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || !ready) return;
    if (!restored.current) {
      restored.current = true;
      const saved = offsets.get(key);
      if (saved) el.scrollTop = saved;
    }
    const onScroll = () => offsets.set(key, el.scrollTop);
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [key, ref, ready]);
}
