import useUiText from "@/i18n/useUiText";
import { FastForward } from "lucide-react";
import { useGame } from "@/context/GameContext";

export default function PriorityHoldControl({ compact = false, battlefield = false }) {
  const ui = useUiText();
  const { state, autoResolveEnabled, autoResolveMode, setAutoResolveMode } = useGame();
  if (compact || state?.mana_payment || state?.decision?.kind === "mana_payment") return null;
  const descriptions = {
    off: "Auto-pass off. Click for timed phase passing.",
    regular: "Auto-pass phases. Click to also resolve when no non-mana actions are playable.",
    safe: "Auto-resolve when no non-mana actions are playable. Click to resolve even with playable actions.",
    all: "Auto-resolve even with playable actions. Click to turn off.",
  };
  return <button type="button" className={battlefield ? "battlefield-auto-pass-toggle" : "player-priority-hold"}
    data-enabled={autoResolveEnabled ? "true" : "false"}
    data-resolve-mode={autoResolveMode}
    aria-label={`${ui("Auto-pass")}: ${ui(descriptions[autoResolveMode])}`} aria-pressed={!!autoResolveEnabled}
    title={ui(descriptions[autoResolveMode])}
    onPointerDown={event => event.stopPropagation()}
    onClick={event => { event.stopPropagation(); setAutoResolveMode(mode => ({ off: "regular", regular: "safe", safe: "all", all: "off" })[mode]); }}>
    {battlefield ? <svg aria-hidden="true" viewBox="0 0 20 20" fill="none">
      <path d="m3.25 4.5 5.5 5.5-5.5 5.5" />
      <path d="m10.25 4.5 5.5 5.5-5.5 5.5" />
    </svg> : <FastForward size={13} aria-hidden="true" />}
    <span className={battlefield ? "sr-only" : undefined}>{ui("Auto-pass")}</span>
  </button>;
}
