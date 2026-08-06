import { useEffect, useRef, useState } from "react";

import { cn } from "@/lib/utils";
import { useMainTitle } from "@/hooks/useMainTitle";
import { useLogStore } from "@/stores/logStore";

const LEVELS = ["ERROR", "WARN", "INFO", "DEBUG"] as const;

function levelClass(level: string): string {
  switch (level) {
    case "ERROR":
      return "text-error";
    case "WARN":
      return "text-warn";
    case "INFO":
      return "text-info";
    default:
      return "text-muted";
  }
}

export function LogViewer() {
  useMainTitle("logs");
  const lines = useLogStore((s) => s.lines);
  const clear = useLogStore((s) => s.clear);
  const [minLevel, setMinLevel] = useState<string>("DEBUG");
  const bottomRef = useRef<HTMLDivElement>(null);

  const threshold = LEVELS.indexOf(minLevel as (typeof LEVELS)[number]);
  const visible = lines.filter(
    (l) => LEVELS.indexOf(l.level as (typeof LEVELS)[number]) <= threshold,
  );

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [visible.length]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-8 shrink-0 items-center justify-between border-b border-subtle px-2">
        <div className="flex gap-px overflow-hidden border border-subtle">
          {LEVELS.map((level) => (
            <button
              key={level}
              type="button"
              onClick={() => setMinLevel(level)}
              className={cn(
                "px-2 py-0.5 text-[10px]",
                minLevel === level
                  ? "bg-raised text-accent"
                  : "text-muted hover:text-secondary",
              )}
            >
              {level.toLowerCase()}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={clear}
          className="text-[11px] text-muted hover:text-error"
        >
          clear
        </button>
      </div>
      <div className="min-h-0 flex-1 select-text overflow-auto p-2 font-mono text-[11px] leading-5">
        {visible.length === 0 ? (
          <p className="text-muted">no log lines yet</p>
        ) : (
          visible.map((line) => (
            <div key={line.id} className="flex gap-2 whitespace-pre-wrap break-all">
              <span className="shrink-0 text-muted">{line.at}</span>
              <span className={cn("w-12 shrink-0", levelClass(line.level))}>
                {line.level}
              </span>
              <span className="shrink-0 text-muted">{shortTarget(line.target)}</span>
              <span className="text-secondary">{line.message}</span>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}

function shortTarget(target: string): string {
  return target.split("::")[0] ?? target;
}
