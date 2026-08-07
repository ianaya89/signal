import { useEffect } from "react";

import { useUiStore } from "@/stores/uiStore";

/** Sets the main pane's header title — and, where a view is a list, how many
 *  rows it is showing — while the view is mounted. */
export function useMainTitle(title: string | undefined, count?: number) {
  const setMainTitle = useUiStore((s) => s.setMainTitle);
  useEffect(() => {
    if (title) setMainTitle(title, count);
  }, [title, count, setMainTitle]);
}
