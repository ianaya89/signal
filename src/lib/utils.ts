import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Human-readable text for anything thrown across the IPC boundary.
 *
 * `SignalError` is tagged `{ kind, message }`, but `message` is only a string
 * for the single-field variants: `InvalidQuery { reason }` serializes its
 * payload as a nested object, so reading `.message` and stringifying it still
 * produced "[object Object]". Every branch here ends at a real string — never
 * render a caught value any other way.
 */
export function errText(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (typeof err !== "object" || err === null) return String(err);

  const { message, kind } = err as { message?: unknown; kind?: unknown };
  if (typeof message === "string" && message) return message;

  // struct variants land here: surface their fields rather than the wrapper
  if (message && typeof message === "object") {
    const parts = Object.values(message as Record<string, unknown>)
      .filter((v) => typeof v !== "object")
      .map(String)
      .filter(Boolean);
    if (parts.length > 0) {
      const body = parts.join(" · ");
      return typeof kind === "string" ? `${kind}: ${body}` : body;
    }
  }

  if (typeof kind === "string" && kind) return kind;

  try {
    const json = JSON.stringify(err);
    if (json && json !== "{}") return json;
  } catch {
    // circular or otherwise unserializable — fall through
  }
  return String(err);
}
