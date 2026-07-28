import { cn } from "@/lib/utils";
import { useToastStore } from "@/stores/toastStore";

const KIND_CLASS = {
  ok: "border-ok text-ok",
  error: "border-error text-error",
  info: "border-info text-info",
} as const;

export function Toasts() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none absolute bottom-14 right-2 z-[90] flex flex-col gap-1">
      {toasts.map((t) => (
        <button
          key={t.id}
          type="button"
          onClick={() => dismiss(t.id)}
          className={cn(
            "pointer-events-auto max-w-80 truncate border bg-surface px-2 py-1 text-left text-[11px]",
            KIND_CLASS[t.kind],
          )}
        >
          {t.message}
        </button>
      ))}
    </div>
  );
}
