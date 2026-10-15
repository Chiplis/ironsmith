import { useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";
import { getPlayerAccent, playerAccentVars } from "@/lib/player-colors";
import { playerDisplayName } from "@/lib/player-display";
import "./perspective-handoff.css";

const VISIBLE_MS = 1600;

// Hotseat handoff: when local automation moves the table to another seat, a
// band in that player's accent sweeps across so the switch never goes unnoticed.
export default function PerspectiveHandoffBanner() {
  const { state, perspectiveHandoff, playerAccentOverrides } = useGame();
  const ui = useUiText();
  const [finishedSeq, setFinishedSeq] = useState(0);
  const shown = perspectiveHandoff && perspectiveHandoff.seq !== finishedSeq ? perspectiveHandoff : null;

  useEffect(() => {
    if (!perspectiveHandoff) return undefined;
    const timer = setTimeout(() => setFinishedSeq(perspectiveHandoff.seq), VISIBLE_MS);
    return () => clearTimeout(timer);
  }, [perspectiveHandoff]);

  if (!shown) return null;
  const players = state?.players || [];
  const name = playerDisplayName(players, shown.player);
  const accent = getPlayerAccent(players, shown.player, null, playerAccentOverrides);
  const subtitle = shown.reason === "turn"
    ? (shown.turn != null ? ui("Turn {0}", { 0: shown.turn }) : ui("Your turn"))
    : shown.reason === "blockers" ? ui("Declare blockers") : ui("Your decision");
  return (
    <div key={shown.seq} className="perspective-handoff" role="status" aria-live="polite"
      style={playerAccentVars(accent)}>
      <div className="perspective-handoff-band">
        <span className="perspective-handoff-eyebrow">{ui("Now playing")}</span>
        <span className="perspective-handoff-name">{name}</span>
        <span className="perspective-handoff-subtitle">{subtitle}</span>
      </div>
    </div>
  );
}
