import { useState } from "react";

import { api } from "@/ipc/invoke";
import { pickFolder } from "@/lib/pickFolder";
import { useScanStore } from "@/stores/scanStore";

export function ScanForm() {
  const [root, setRoot] = useState("~/Music");
  const start = useScanStore((s) => s.start);
  const fail = useScanStore((s) => s.fail);
  const lastError = useScanStore((s) => s.lastError);

  const scan = async (path: string) => {
    start();
    try {
      await api.scanLibrary(path);
    } catch (err) {
      fail(errText(err));
    }
  };

  const browse = async () => {
    const folder = await pickFolder();
    if (folder) {
      setRoot(folder);
      await scan(folder);
    }
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-3">
      <p className="text-muted">library empty — point signal at your music</p>
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void scan(root);
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
        <button
          type="button"
          onClick={() => void browse()}
          className="border border-subtle bg-raised px-3 py-1 text-secondary hover:border-focus hover:text-accent"
        >
          browse…
        </button>
      </form>
      <p className="max-w-96 text-center text-[11px] text-muted">
        for iCloud Drive or other protected folders use browse… — macOS only
        grants access through the native picker
      </p>
      {lastError && (
        <p className="max-w-[480px] text-center text-[12px] text-error">
          {lastError}
        </p>
      )}
    </div>
  );
}

function errText(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}
