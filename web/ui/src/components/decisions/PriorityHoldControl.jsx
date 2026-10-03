import useUiText from "@/i18n/useUiText";
import { useRef } from "react";
import { Hand, FastForward } from "lucide-react";
import { useGame } from "@/context/GameContext";

export default function PriorityHoldControl() {
  const ui = useUiText();
  const { holdRule, setHoldRule, autoResolveEnabled, setAutoResolveEnabled } = useGame();
  const previousRule = useRef("never");
  const holding = holdRule === "always";

  return (
    <>
      <button
        type="button"
        className="player-priority-hold"
        aria-pressed={holding}
        title={ui(holding
          ? "Automatic priority passing is paused. Click to restore your previous hold setting."
          : "Hold priority until turned off, including after casting your own spells. Enable before casting.")}
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
        <span>{holding ? ui("Holding priority") : ui("Hold priority")}</span>
      </button>
      <button
        type="button"
        className="player-priority-hold"
        aria-label={ui("Auto-pass")}
        aria-pressed={!!autoResolveEnabled}
        title={ui("Automatically resolve whenever you have priority and the stack is not empty.")}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => {
          event.stopPropagation();
          setAutoResolveEnabled((enabled) => !enabled);
        }}
      >
        <FastForward size={13} aria-hidden="true" />
        <span>{ui("Auto-pass")}</span>
      </button>
    </>
  );
}
