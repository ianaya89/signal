import { useState } from "react";

import { api } from "@/ipc/invoke";
import { useScanStore } from "@/stores/scanStore";

export function ScanForm() {
  const [root, setRoot] = useState("~/Music");
  const [error, setError] = useState<string | null>(null);
  const start = useScanStore((s) => s.start);

  const submit = async () => {
    setError(null);
    try {
      await api.scanLibrary(root);
      start();
    } catch (err) {
      setError(
        typeof err === "object" && err !== null && "message" in err
          ? String((err as { message: unknown }).message)
          : String(err),
      );
    }
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-3">
      <p className="text-muted">library empty — point signal at your music</p>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <input
          value={root}
          onChange={(e) => setRoot(e.target.value)}
          spellCheck={false}
          className="w-72 border border-subtle bg-base px-2 py-1 text-primary outline-none focus:border-focus"
        />
        <button
          type="submit"
          className="border border-subtle bg-raised px-3 py-1 text-secondary hover:border-focus hover:text-accent"
        >
          scan
        </button>
      </form>
      {error && <p className="text-error text-[12px]">{error}</p>}
    </div>
  );
}
