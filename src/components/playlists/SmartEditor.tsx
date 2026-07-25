import { useState } from "react";

import type { SmartCondition, SmartRules } from "@/ipc/types";
import { cn } from "@/lib/utils";

// mirror of the backend whitelist (signal-db/src/smart.rs)
const FIELDS = [
  "title",
  "artist_name",
  "album_name",
  "genre",
  "year",
  "rating",
  "favorite",
  "play_count",
  "skip_count",
  "added_at",
  "codec",
  "bit_depth",
  "sample_rate_hz",
  "duration_ms",
] as const;

const OPS = [
  "eq",
  "neq",
  "gt",
  "gte",
  "lt",
  "lte",
  "contains",
  "not_contains",
  "within_days",
  "is_null",
  "is_not_null",
] as const;

const ORDER_FIELDS = [
  "added_at",
  "last_played_at",
  "play_count",
  "year",
  "rating",
  "sample_rate_hz",
  "bit_depth",
  "duration_ms",
  "title",
] as const;

const NUMERIC_FIELDS = new Set([
  "year",
  "rating",
  "play_count",
  "skip_count",
  "bit_depth",
  "sample_rate_hz",
  "duration_ms",
]);

function coerceValue(field: string, raw: string): string | number | boolean {
  if (field === "favorite") return raw === "true";
  if (NUMERIC_FIELDS.has(field)) {
    const n = Number(raw);
    return Number.isFinite(n) ? n : 0;
  }
  return raw;
}

export function SmartEditor({
  initialName = "",
  initialRules,
  onSave,
  onCancel,
}: {
  initialName?: string;
  initialRules?: SmartRules;
  onSave: (name: string, rules: SmartRules) => void | Promise<void>;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initialName);
  const [match, setMatch] = useState<"all" | "any">(initialRules?.match ?? "all");
  const [conditions, setConditions] = useState<SmartCondition[]>(
    initialRules?.conditions ?? [{ field: "play_count", op: "eq", value: 0 }],
  );
  const [orderBy, setOrderBy] = useState(initialRules?.order_by ?? "added_at");
  const [orderDir, setOrderDir] = useState<"asc" | "desc">(
    initialRules?.order_dir ?? "desc",
  );

  const update = (i: number, patch: Partial<SmartCondition>) => {
    setConditions((cs) =>
      cs.map((c, j) => {
        if (j !== i) return c;
        const next = { ...c, ...patch };
        // keep value type in sync with the field
        if (patch.field) next.value = coerceValue(patch.field, String(next.value));
        return next;
      }),
    );
  };

  const save = () => {
    if (!name.trim() || conditions.length === 0) return;
    void onSave(name.trim(), {
      match,
      conditions,
      order_by: orderBy,
      order_dir: orderDir,
      limit: null,
    });
  };

  return (
    <div className="flex flex-col gap-2 border border-focus bg-raised p-3">
      <div className="flex items-center gap-2">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="smart playlist name…"
          spellCheck={false}
          className="w-56 border border-subtle bg-base/60 px-2 py-1 text-[12px] text-primary outline-none focus:border-focus"
        />
        <span className="text-[11px] text-muted">match</span>
        <div className="flex gap-px overflow-hidden border border-subtle">
          {(["all", "any"] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMatch(m)}
              className={cn(
                "px-2 py-0.5 text-[11px]",
                match === m ? "bg-surface text-accent" : "text-muted",
              )}
            >
              {m}
            </button>
          ))}
        </div>
      </div>

      {conditions.map((cond, i) => (
        <div key={i} className="flex items-center gap-1">
          <select
            value={cond.field}
            onChange={(e) => update(i, { field: e.target.value })}
            className={SELECT}
          >
            {FIELDS.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
          <select
            value={cond.op}
            onChange={(e) => update(i, { op: e.target.value })}
            className={SELECT}
          >
            {OPS.map((op) => (
              <option key={op} value={op}>
                {op}
              </option>
            ))}
          </select>
          {cond.op !== "is_null" && cond.op !== "is_not_null" && (
            <input
              value={String(cond.value)}
              onChange={(e) =>
                update(i, { value: coerceValue(cond.field, e.target.value) })
              }
              spellCheck={false}
              className="w-32 border border-subtle bg-base/60 px-1.5 py-0.5 text-[11px] text-primary outline-none focus:border-focus"
            />
          )}
          <button
            type="button"
            onClick={() => setConditions((cs) => cs.filter((_, j) => j !== i))}
            disabled={conditions.length === 1}
            className="px-1 text-[12px] text-muted hover:text-error disabled:opacity-30"
          >
            ✕
          </button>
        </div>
      ))}

      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() =>
            setConditions((cs) => [...cs, { field: "year", op: "gte", value: 2000 }])
          }
          className="text-[11px] text-secondary hover:text-accent"
        >
          + condition
        </button>
        <span className="ml-3 text-[11px] text-muted">order by</span>
        <select value={orderBy} onChange={(e) => setOrderBy(e.target.value)} className={SELECT}>
          {ORDER_FIELDS.map((f) => (
            <option key={f} value={f}>
              {f}
            </option>
          ))}
        </select>
        <select
          value={orderDir}
          onChange={(e) => setOrderDir(e.target.value as "asc" | "desc")}
          className={SELECT}
        >
          <option value="desc">desc</option>
          <option value="asc">asc</option>
        </select>
      </div>

      <div className="mt-1 flex gap-2">
        <button
          type="button"
          onClick={save}
          disabled={!name.trim()}
          className="border border-subtle bg-surface px-3 py-1 text-[11px] text-accent hover:border-focus disabled:opacity-40"
        >
          save
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="border border-subtle px-3 py-1 text-[11px] text-muted hover:text-secondary"
        >
          cancel
        </button>
      </div>
    </div>
  );
}

const SELECT =
  "border border-subtle bg-base/60 px-1 py-0.5 text-[11px] text-secondary outline-none focus:border-focus";
