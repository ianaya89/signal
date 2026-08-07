// Single source of truth for the ? help overlay. Keep in sync with the
// dispatcher in AppShell.

export interface Binding {
  keys: string;
  action: string;
}

export const BINDING_GROUPS: { title: string; bindings: Binding[] }[] = [
  {
    title: "playback",
    bindings: [
      { keys: "space", action: "play / pause" },
      { keys: "{ / }", action: "previous / next track" },
      { keys: "[ / ]", action: "seek -5s / +5s" },
      { keys: "= / -", action: "volume up / down" },
      { keys: "m", action: "mute / unmute" },
    ],
  },
  {
    title: "navigate",
    bindings: [
      { keys: "j / k  ·  ↓ / ↑", action: "move down / up" },
      { keys: "h / l  ·  ← / →", action: "move sideways (album grid)" },
      { keys: "gg / G  ·  home / end", action: "jump to top / bottom" },
      { keys: ".", action: "jump to what is playing" },
      { keys: "enter", action: "play from here" },
      { keys: "esc", action: "back" },
      { keys: "tab / shift+tab", action: "cycle panes" },
      { keys: "1 / 2 / 3", action: "focus library / main / inspector" },
      { keys: "/", action: "search" },
      { keys: "ctrl+p / cmd+k", action: "command palette" },
      { keys: "S / L / D", action: "stats / logs / discover" },
      { keys: "F", action: "favorites" },
    ],
  },
  {
    title: "library",
    bindings: [
      { keys: "a", action: "stage track to queue" },
      { keys: "x", action: "remove (queue / playlist)" },
      { keys: "f", action: "toggle favorite" },
      { keys: "r then 0-5", action: "rate track (0 clears)" },
    ],
  },
  {
    title: "layout",
    bindings: [
      { keys: "b", action: "toggle library pane" },
      { keys: "i", action: "toggle inspector pane" },
      { keys: "M", action: "mini player" },
      { keys: "P", action: "pulse mode" },
      { keys: "esc (compact)", action: "back to full window" },
      { keys: "?", action: "this help" },
    ],
  },
];
