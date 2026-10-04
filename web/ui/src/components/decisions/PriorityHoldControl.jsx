import useUiText from "@/i18n/useUiText";
import { useRef } from "react";
import { Hand, FastForward } from "lucide-react";
import { useGame } from "@/context/GameContext";

export default function PriorityHoldControl({ compact = false, previousRuleRef = null }) {
  const ui = useUiText();
  const { holdRule, setHoldRule, autoResolveEnabled, setAutoResolveEnabled } = useGame();
  const localPreviousRule = useRef("never");
  const previousRule = previousRuleRef || localPreviousRule;
  const holding = holdRule === "always";
  const holdLabel = ui(holding ? "Holding priority" : "Hold priority");
  const autoResolveLabel = ui("Auto-pass");

  return (
    <>
      <button
        type="button"
        className={`player-priority-hold${compact ? " player-priority-hold--compact" : ""}`}
        aria-pressed={holding}
        aria-label={holdLabel}
        title={compact ? holdLabel : ui(holding
          ? "Automatic priority passing is paused. Click to restore your previous hold setting."
          : "Hold priority until turned off, including after casting your own spells. Enable before casting.")}
        data-tooltip={compact ? holdLabel : undefined}
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
        <span className={compact ? "sr-only" : undefined}>{holdLabel}</span>
      </button>
      {/* The focused desktop HUD draws its own auto-pass toggle beside this
          control, so the compact variant only renders the hold button. */}
      {!compact ? (
        <button
          type="button"
          className="player-priority-hold"
          aria-label={autoResolveLabel}
          aria-pressed={!!autoResolveEnabled}
          title={ui("Automatically resolve whenever you have priority and the stack is not empty.")}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            setAutoResolveEnabled((enabled) => !enabled);
          }}
        >
          <FastForward size={13} aria-hidden="true" />
          <span>{autoResolveLabel}</span>
        </button>
      ) : null}
    </>
  );
}
