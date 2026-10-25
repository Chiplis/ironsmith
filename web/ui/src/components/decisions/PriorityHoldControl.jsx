import useUiText from "@/i18n/useUiText";
import { Clock, Hand } from "lucide-react";
import { useGame } from "@/context/GameContext";

export default function PriorityHoldControl({ compact = false, battlefield = false }) {
  const ui = useUiText();
  const { state, autoResolveEnabled, autoResolveMode, setAutoResolveMode, holdRule, setHoldRule } = useGame();
  if (compact || state?.mana_payment || state?.decision?.kind === "mana_payment") return null;
  const descriptions = {
    off: "Count down whenever the stack is nonempty. Click to keep spells manual.",
    safe: "Keep Resolve available while any spell is on the stack. Click to disable all priority countdowns.",
    all: "Priority countdowns disabled. Click to count down for every stack item.",
  };
  const fullControl = holdRule === "always";
  return <><button type="button" className="player-full-control" aria-pressed={fullControl} aria-label={ui("Full control")}
    title={ui("Full control: manually confirm every priority and decision")}
    onPointerDown={event => event.stopPropagation()}
    onClick={event => { event.stopPropagation(); setHoldRule(fullControl ? "never" : "always"); }}>
    <Hand size={14} aria-hidden="true" /><span className={battlefield ? "sr-only" : undefined}>{ui("Full control")}</span>
  </button><button type="button" className={battlefield ? "battlefield-auto-pass-toggle" : "player-priority-hold"}
    data-enabled={autoResolveEnabled ? "true" : "false"}
    data-resolve-mode={autoResolveMode}
    aria-label={`${ui("Auto-pass")}: ${ui(descriptions[autoResolveMode])}`} aria-pressed={!!autoResolveEnabled}
    title={ui(descriptions[autoResolveMode])}
    onPointerDown={event => event.stopPropagation()}
    onClick={event => { event.stopPropagation(); setAutoResolveMode(mode => ({ off: "safe", safe: "all", all: "off" })[mode]); }}>
    <Clock size={battlefield ? 20 : 13} aria-hidden="true" />
    <span className={battlefield ? "sr-only" : undefined}>{ui("Auto-pass")}</span>
  </button></>;
}
