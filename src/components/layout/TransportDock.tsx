import {
  ModeButtons,
  Timeline,
  TransportControls,
  VolumeSlider,
} from "@/components/player/TransportBar";
import { HeartEqualizer } from "@/components/ui/HeartEqualizer";
import { usePlayerStore } from "@/stores/playerStore";

/** Bottom transport dock: brand mark, core transport, timeline, modes,
 *  volume. Mirrors the pane margins so it lines up with the columns above. */
export function TransportDock() {
  const status = usePlayerStore((s) => s.status);

  return (
    <footer className="mx-2 mb-2 mt-1.5 flex h-9 shrink-0 items-center gap-3 border border-subtle bg-surface px-2 text-[13px]">
      <span className="pointer-events-none flex shrink-0 items-center gap-1.5 text-[11px]">
        <HeartEqualizer size={16} playing={status === "playing"} />
        <span className="text-accent">❯</span>{" "}
        <span className="text-secondary">signal</span>
      </span>
      <TransportControls />
      <Timeline className="flex-1" />
      <span className="flex shrink-0 items-center gap-3 text-[11px]">
        <ModeButtons />
        <VolumeSlider />
      </span>
    </footer>
  );
}
