/**
 * Section labels for a sorted list.
 *
 * Grouping only ever *describes* the order the list is already in — it never
 * reorders anything. That is why each grouper is paired with the sort it
 * belongs to and why some sorts have none: a header whose rows are not
 * contiguous is worse than no header at all.
 *
 * Initials are taken from the raw first character, articles included, because
 * the backend sorts on the raw name too (`ORDER BY name COLLATE NOCASE`).
 * Folding "The Beatles" under B while the list sorts it under T would split
 * the group in two.
 */

/** First character, with digits, punctuation and anything non-latin as "#". */
export function initialOf(name: string): string {
  const ch = name.trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(ch) ? ch : "#";
}

export function decadeOf(year: number | null): string {
  if (!year) return "no year";
  return `${Math.floor(year / 10) * 10}s`;
}

export interface Group {
  label: string;
  /** Index of this group's first row in the flat list. */
  start: number;
  count: number;
}

/**
 * Runs of consecutive items sharing a label. Returns an empty array when
 * `label` is null, which is how a sort opts out of grouping.
 */
export function groupRuns<T>(
  items: T[],
  label: ((item: T) => string) | null,
): Group[] {
  if (!label) return [];
  const groups: Group[] = [];
  for (const [index, item] of items.entries()) {
    const current = label(item);
    const last = groups.at(-1);
    if (last && last.label === current) {
      last.count += 1;
    } else {
      groups.push({ label: current, start: index, count: 1 });
    }
  }
  return groups;
}

/** Group label to render before row `index`, if one starts there. */
export function headerAt(groups: Group[], index: number): Group | undefined {
  return groups.find((g) => g.start === index);
}

/**
 * Whether headers earn their place.
 *
 * Sections that mostly hold a single row are noise rather than structure — on
 * a small library, grouping artists by initial produced fourteen headers for
 * twenty-six rows. Requiring two rows per section on average means grouping
 * appears once a list is long enough to need it and stays out of the way until
 * then, which also makes it self-scaling: the header count is capped by the
 * alphabet while the rows are not.
 */
export function worthGrouping<T>(groups: Group[], items: T[]): Group[] {
  if (groups.length < 2) return [];
  return items.length / groups.length >= 2 ? groups : [];
}
