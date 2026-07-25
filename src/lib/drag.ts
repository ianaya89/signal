import { getCurrentWindow } from "@tauri-apps/api/window";

/** Manual window drag: works from any inert spot regardless of nesting
 *  (data-tauri-drag-region only fires on the exact element carrying it).
 *  Interactive elements and anything marked data-no-drag are excluded. */
export function dragWindow(e: React.MouseEvent) {
  if (e.button !== 0) return;
  const target = e.target as HTMLElement;
  if (target.closest("button, input, a, select, textarea, [data-no-drag]")) {
    return;
  }
  void getCurrentWindow().startDragging();
}
