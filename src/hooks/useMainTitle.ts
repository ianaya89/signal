import { useEffect } from "react";

import { useUiStore } from "@/stores/uiStore";

/** Sets the main pane's header title while the view is mounted. */
export function useMainTitle(title: string | undefined) {
  const setMainTitle = useUiStore((s) => s.setMainTitle);
  useEffect(() => {
    if (title) setMainTitle(title);
  }, [title, setMainTitle]);
}
