import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import { isEffectOrderingDecision } from "@/lib/effect-ordering";

// Both desktop appearances share the rendered card pile. Phones and effect
// ordering keep the rail, which provides the required reorder controls.
export default function useCardStackMode() {
  const { state } = useGame();
  const { nonDesktopViewport } = useViewportLayout();
  return !nonDesktopViewport
    && !isEffectOrderingDecision(state?.decision);
}
