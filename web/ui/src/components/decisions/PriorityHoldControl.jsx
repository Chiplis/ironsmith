import useUiText from "@/i18n/useUiText";
import { FastForward } from "lucide-react";
import { useGame } from "@/context/GameContext";

export default function PriorityHoldControl({ compact = false }) {
  const ui = useUiText();
  const { autoResolveEnabled, setAutoResolveEnabled } = useGame();
  if (compact) return null;
  return <button type="button" className="player-priority-hold"
    aria-label={ui("Auto-pass")} aria-pressed={!!autoResolveEnabled}
    title={ui("Show a timed Hold Priority button after casting or activating. Turn off to resolve manually.")}
    onPointerDown={event => event.stopPropagation()}
    onClick={event => { event.stopPropagation(); setAutoResolveEnabled(enabled => !enabled); }}>
    <FastForward size={13} aria-hidden="true" /><span>{ui("Auto-pass")}</span>
  </button>;
}
