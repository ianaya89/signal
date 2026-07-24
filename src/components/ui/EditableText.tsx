import { useEffect, useRef, useState } from "react";

import { cn } from "@/lib/utils";

/** Inline-editable label: pencil on hover, Enter saves, Esc cancels. */
export function EditableText({
  value,
  onSave,
  className,
  inputClassName,
}: {
  value: string;
  onSave: (next: string) => void | Promise<void>;
  className?: string;
  inputClassName?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) {
      setDraft(value);
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  }, [editing, value]);

  const commit = async () => {
    setEditing(false);
    const next = draft.trim();
    if (next && next !== value) {
      await onSave(next);
    }
  };

  // stop row-level Link/double-click handlers from firing while editing
  const swallow = (e: React.SyntheticEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  if (editing) {
    return (
      <input
        ref={inputRef}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => void commit()}
        onClick={swallow}
        onMouseDown={(e) => e.stopPropagation()}
        onDoubleClick={swallow}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") void commit();
          if (e.key === "Escape") setEditing(false);
        }}
        spellCheck={false}
        className={cn(
          "border border-focus bg-base/60 px-1 outline-none",
          inputClassName ?? className,
        )}
      />
    );
  }

  return (
    <span className={cn("group/edit inline-flex min-w-0 items-center gap-1", className)}>
      <span className="truncate">{value}</span>
      <button
        type="button"
        onClick={(e) => {
          swallow(e);
          setEditing(true);
        }}
        onDoubleClick={swallow}
        title="rename"
        className="invisible shrink-0 text-[10px] text-muted hover:text-accent group-hover/edit:visible"
      >
        ✎
      </button>
    </span>
  );
}
