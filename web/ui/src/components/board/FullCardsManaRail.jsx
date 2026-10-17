import { createPortal } from "react-dom";
import ManaPool from "@/components/left-rail/ManaPool";
import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";

// The local mana pool of the desktop HUD, in both battlefield appearances:
// a left rail mirroring the opponent's. The action dock carries its own copy,
// but the dock animates its position with translate/scale, which would drag
// any fixed child along to the right; rendered here, the rail stays put.
export default function FullCardsManaRail() {
  const { state } = useGame();
  const { nonDesktopViewport, tabletCompactViewport } = useViewportLayout();
  const players = state?.players || [];
  const me = players.find((player) => player.id === state?.perspective) || null;
  if (nonDesktopViewport || tabletCompactViewport || !me) return null;
  if (typeof document === "undefined") return null;
  return createPortal(
    <div className="fc-local-mana-rail" aria-label="Your mana pool">
      <ManaPool pool={me.mana_pool} player={me} alwaysVisible compact activeScale={2} />
    </div>,
    document.body,
  );
}
