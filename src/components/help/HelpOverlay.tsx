import { BINDING_GROUPS } from "@/lib/bindings";
import { useKeyboardStore } from "@/lib/keyboard";

export function HelpOverlay() {
  const mode = useKeyboardStore((s) => s.mode);
  const setMode = useKeyboardStore((s) => s.setMode);

  if (mode !== "help") return null;

  return (
    <div
      className="absolute inset-0 z-50 flex items-center justify-center bg-black/40"
      onMouseDown={() => setMode("normal")}
    >
      <div
        className="max-h-[80vh] w-[640px] overflow-auto border border-focus bg-raised p-4"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="mb-3 flex items-center justify-between">
          <span className="text-[13px] text-accent">[ keyboard reference ]</span>
          <span className="text-[11px] text-muted">esc to close</span>
        </div>
        <div className="grid grid-cols-2 gap-x-8 gap-y-4">
          {BINDING_GROUPS.map((group) => (
            <section key={group.title}>
              <h3 className="mb-1 text-[10px] uppercase tracking-wider text-muted">
                {group.title}
              </h3>
              <ul className="flex flex-col gap-0.5">
                {group.bindings.map((b) => (
                  <li key={b.keys} className="flex items-baseline gap-2 text-[11px]">
                    <kbd className="w-28 shrink-0 bg-base px-1 py-0.5 text-accent">
                      {b.keys}
                    </kbd>
                    <span className="text-secondary">{b.action}</span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
