import { errText } from "@/lib/utils";

/**
 * The three things every list view says when it has no rows to show.
 *
 * These were written out by hand in ~35 places and had drifted across five
 * different size/colour combinations, so "loading" looked like a different
 * kind of message depending on which pane you were in.
 */

export function Loading({ label = "loading…" }: { label?: string }) {
  return <p className="p-3 text-[12px] text-muted">{label}</p>;
}

export function Empty({ children }: { children: React.ReactNode }) {
  return <p className="p-3 text-[12px] text-muted">{children}</p>;
}

/** Always routed through `errText` — a raw IPC error renders as [object Object]. */
export function Failed({
  error,
  prefix,
}: {
  error: unknown;
  prefix?: string;
}) {
  return (
    <p className="p-3 text-[12px] text-error">
      {prefix ? `${prefix} — ` : ""}
      {errText(error)}
    </p>
  );
}
