import { useGame } from "@/context/GameContext";
import useBattlefieldAppearance from "@/hooks/useBattlefieldAppearance";
import useViewportLayout from "@/hooks/useViewportLayout";
import { isEffectOrderingDecision } from "@/lib/effect-ordering";

// The "Compact artwork" appearance shows the stack as a pile of rendered
// cards over the board (CardStackPile) instead of the scrolling rail. Phones
// keep their rails, and ordering triggers or replacements keeps the rail too:
// its reorder controls have no place on a pile.
export default function useCardStackMode() {
  const [appearance] = useBattlefieldAppearance();
  const { state } = useGame();
  const { nonDesktopViewport } = useViewportLayout();
  return appearance.compactCards
    && !nonDesktopViewport
    && !isEffectOrderingDecision(state?.decision);
}
