import { useRef } from "react";
import { Hand } from "lucide-react";
import { useGame } from "@/context/GameContext";

export default function PriorityHoldControl() {
  const { holdRule, setHoldRule } = useGame();
  const previousRule = useRef("never");
  const holding = holdRule === "always";

  return (
    <button
      type="button"
      className="player-priority-hold"
      aria-pressed={holding}
      title={holding
        ? "Automatic priority passing is paused. Click to restore your previous hold setting."
        : "Hold priority until turned off, including after casting your own spells. Enable before casting."}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        if (holding) {
          setHoldRule(previousRule.current);
        } else {
          previousRule.current = holdRule || "never";
          setHoldRule("always");
        }
      }}
    >
      <Hand size={13} aria-hidden="true" />
      <span>{holding ? "Holding priority" : "Hold priority"}</span>
    </button>
  );
}
