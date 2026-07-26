import { useEffect, useState } from "react";

interface Tip {
  text: string;
  x: number;
  y: number;
  below: boolean;
}

/** WKWebView never shows native `title` tooltips, so this layer renders
 *  them: hovering any [title] element moves the text into data-tip
 *  (suppressing any native attempt) and shows a styled bubble. */
export function TooltipLayer() {
  const [tip, setTip] = useState<Tip | null>(null);

  useEffect(() => {
    let timer: number | undefined;
    let current: HTMLElement | null = null;

    const hide = () => {
      window.clearTimeout(timer);
      timer = undefined;
      current = null;
      setTip(null);
    };

    const onOver = (e: MouseEvent) => {
      const el = (e.target as HTMLElement | null)?.closest?.(
        "[title], [data-tip]",
      ) as HTMLElement | null;
      if (!el || el === current) return;
      window.clearTimeout(timer);
      setTip(null);
      current = el;
      const title = el.getAttribute("title");
      if (title) {
        el.dataset.tip = title;
        el.removeAttribute("title");
      }
      const text = el.dataset.tip;
      if (!text) {
        current = null;
        return;
      }
      timer = window.setTimeout(() => {
        const r = el.getBoundingClientRect();
        const below = r.top < 48;
        setTip({
          text,
          x: r.left + r.width / 2,
          y: below ? r.bottom + 6 : r.top - 6,
          below,
        });
      }, 350);
    };

    const onOut = (e: MouseEvent) => {
      if (!current) return;
      const to = e.relatedTarget as HTMLElement | null;
      if (to && current.contains(to)) return;
      hide();
    };

    document.addEventListener("mouseover", onOver);
    document.addEventListener("mouseout", onOut);
    document.addEventListener("mousedown", hide, true);
    return () => {
      document.removeEventListener("mouseover", onOver);
      document.removeEventListener("mouseout", onOut);
      document.removeEventListener("mousedown", hide, true);
    };
  }, []);

  if (!tip) return null;
  return (
    <div
      className="pointer-events-none fixed z-[200] whitespace-nowrap border border-focus bg-raised px-1.5 py-0.5 text-[10px] text-secondary"
      style={{
        left: Math.min(Math.max(tip.x, 60), window.innerWidth - 60),
        top: tip.y,
        transform: `translate(-50%, ${tip.below ? "0" : "-100%"})`,
      }}
    >
      {tip.text}
    </div>
  );
}
