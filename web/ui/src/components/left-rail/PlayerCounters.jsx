import useUiText from "@/i18n/useUiText";
import { playerCounters } from "@/lib/player-counters";
import "./player-counters.css";

export default function PlayerCounters({ player }) {
  const ui = useUiText();
  const counters = playerCounters(player);
  if (!counters.length) return null;
  return (
    <span className="player-counters">
      {counters.map(({ kind, amount }) => (
        <span key={kind} className="player-counter-chip" title={`${ui(kind)}: ${amount}`}>
          <span>{ui(kind)}</span><strong>{amount}</strong>
        </span>
      ))}
    </span>
  );
}
