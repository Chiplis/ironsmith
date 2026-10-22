import { CloudLightning } from "lucide-react";
import { counterSymbolUrl } from "@/lib/mana-assets";
import useUiText from "@/i18n/useUiText";
import { playerCounters } from "@/lib/player-counters";
import "./player-counters.css";

export default function PlayerCounters({ player, stormCount = 0 }) {
  const ui = useUiText();
  const counters = [
    ...(Number.isFinite(Number(stormCount)) && Number(stormCount) > 0
      ? [{ kind: "storm", amount: Number(stormCount) }] : []),
    ...playerCounters(player),
  ];
  if (!counters.length) return null;
  return (
    <span className="player-counters">
      {counters.map(({ kind, amount }) => (
        <span key={kind} className="player-counter-chip" title={`${ui(kind)}: ${amount}`} aria-label={`${ui(kind)}: ${amount}`} data-counter-kind={kind}>
          {kind === "storm" ? <CloudLightning size={22} aria-hidden="true" />
            : counterSymbolUrl(kind) ? <img src={counterSymbolUrl(kind)} width={22} height={22} alt="" aria-hidden="true" />
              : <span>{ui(kind)}</span>}
          <strong>{amount}</strong>
        </span>
      ))}
    </span>
  );
}
